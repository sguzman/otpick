mod app;
mod clipboard;
mod ingest;
mod model;
mod paths;
mod search;
mod session;
mod totp;
mod vault;

use std::error::Error;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::PathBuf;

use app::PickerApp;
use eframe::egui;
use model::Account;
use totp_rs::Secret;
use vault::Vault;
use zeroize::Zeroizing;

fn main() {
    if let Err(error) = run() {
        eprintln!("otpick: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);

    match args.next().as_deref() {
        Some("-h" | "--help" | "help") => print_help(),
        Some("-V" | "--version") => println!("otpick {}", env!("CARGO_PKG_VERSION")),
        Some("init") => init_vault()?,
        Some("unlock") => unlock_vault()?,
        Some("lock") => lock_vault()?,
        Some("status") => status()?,
        Some("list") => list_accounts()?,
        Some("add") => add_account()?,
        Some("import") => {
            let source = args.next().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "import requires a file path or - for stdin",
                )
            })?;
            if args.next().is_some() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "import accepts exactly one source",
                )
                .into());
            }
            import_accounts(&source)?;
        }
        Some(other) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unknown command: {other}"),
            )
            .into());
        }
        None => run_picker()?,
    }

    Ok(())
}

fn run_picker() -> eframe::Result {
    let (accounts, notice) = load_picker_accounts();

    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: egui::ViewportBuilder::default()
            .with_app_id("otpick")
            .with_title("OTPick")
            .with_inner_size([520.0, 300.0])
            .with_resizable(false)
            .with_decorations(false)
            .with_always_on_top(),
        ..Default::default()
    };

    eframe::run_native(
        "OTPick",
        options,
        Box::new(move |cc| Ok(Box::new(PickerApp::new(cc, accounts, notice)))),
    )
}

fn load_picker_accounts() -> (Vec<Account>, Option<String>) {
    let path = match paths::vault_path() {
        Ok(path) => path,
        Err(error) => return (Vec::new(), Some(error.to_string())),
    };

    if !path.exists() {
        return (
            Vec::new(),
            Some("No vault yet. Run otpick init once.".to_owned()),
        );
    }

    let key = match session::load() {
        Ok(Some(key)) => key,
        Ok(None) => {
            return (
                Vec::new(),
                Some("Vault locked. Run otpick unlock once for this login session.".to_owned()),
            );
        }
        Err(error) => return (Vec::new(), Some(error.to_string())),
    };

    match Vault::open_with_key(&path, key) {
        Ok(vault) => (vault.into_accounts(), None),
        Err(error) => {
            let _ = session::clear();
            (
                Vec::new(),
                Some(format!(
                    "{error}. Session key cleared; run otpick unlock again."
                )),
            )
        }
    }
}

fn init_vault() -> Result<(), Box<dyn Error>> {
    let path = paths::vault_path()?;
    let passphrase = new_passphrase()?;

    let vault = Vault::create(&path, passphrase.as_bytes())?;
    session::store(vault.key())?;

    println!("Initialized and unlocked {}", path.display());
    Ok(())
}

fn unlock_vault() -> Result<(), Box<dyn Error>> {
    let path = paths::vault_path()?;
    let passphrase = Zeroizing::new(rpassword::prompt_password("OTPick passphrase: ")?);
    let vault = Vault::unlock(&path, passphrase.as_bytes())?;

    session::store(vault.key())?;
    println!("Unlocked for this login session.");
    Ok(())
}

fn lock_vault() -> Result<(), Box<dyn Error>> {
    if session::clear()? {
        println!("Locked.");
    } else {
        println!("Already locked.");
    }
    Ok(())
}

fn status() -> Result<(), Box<dyn Error>> {
    let path = paths::vault_path()?;
    println!("Vault: {}", path.display());
    println!("Exists: {}", path.exists());
    println!("Session unlocked: {}", session::load()?.is_some());
    Ok(())
}

fn list_accounts() -> Result<(), Box<dyn Error>> {
    let (_, vault) = open_unlocked_vault()?;

    for account in vault.accounts() {
        println!("{}", account.label());
    }

    Ok(())
}

fn add_account() -> Result<(), Box<dyn Error>> {
    let issuer = prompt_line("Issuer: ")?;
    let account_name = prompt_line("Account: ")?;
    let secret = Zeroizing::new(rpassword::prompt_password("Base32 TOTP secret: ")?);

    let account = ingest::account_from_base32(issuer, account_name, secret.as_str())?;
    persist_accounts(vec![account])?;
    println!("Added account.");
    Ok(())
}

fn import_accounts(source: &str) -> Result<(), Box<dyn Error>> {
    let mut input = String::new();

    if source == "-" {
        io::stdin().read_to_string(&mut input)?;
    } else {
        File::open(source)?.read_to_string(&mut input)?;
    }

    let input = Zeroizing::new(input);
    let accounts = ingest::parse_otpauth_document(input.as_str())?;
    let count = accounts.len();

    persist_accounts(accounts)?;
    println!("Imported {count} account(s).");
    Ok(())
}

fn persist_accounts(accounts: Vec<Account>) -> Result<(), Box<dyn Error>> {
    let (path, mut vault) = open_unlocked_vault()?;
    ingest::ensure_unique(vault.accounts(), &accounts)?;

    vault.accounts_mut().extend(accounts);
    vault.save(&path)?;
    Ok(())
}

fn open_unlocked_vault() -> Result<(PathBuf, Vault), Box<dyn Error>> {
    let path = paths::vault_path()?;
    let key = session::load()?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            "vault is locked; run otpick unlock",
        )
    })?;
    let vault = Vault::open_with_key(&path, key)?;
    Ok((path, vault))
}

fn prompt_line(prompt: &str) -> io::Result<String> {
    print!("{prompt}");
    io::stdout().flush()?;

    let mut value = String::new();
    io::stdin().read_line(&mut value)?;

    Ok(value.trim().to_owned())
}

fn new_passphrase() -> Result<Zeroizing<String>, Box<dyn Error>> {
    let first = Zeroizing::new(rpassword::prompt_password("New OTPick passphrase: ")?);
    if first.is_empty() {
        return Err(
            io::Error::new(io::ErrorKind::InvalidInput, "passphrase cannot be empty").into(),
        );
    }

    let second = Zeroizing::new(rpassword::prompt_password("Confirm passphrase: ")?);
    if first.as_str() != second.as_str() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "passphrases do not match").into());
    }

    Ok(first)
}

fn print_help() {
    println!("OTPick - one-shot TOTP picker");
    println!();
    println!("Usage:");
    println!("  otpick             Open the picker");
    println!("  otpick init        Create an encrypted vault and unlock it");
    println!("  otpick unlock      Unlock the vault for this login session");
    println!("  otpick lock        Remove the session key");
    println!("  otpick status      Show vault and session state");
    println!("  otpick list        List account labels without exposing codes");
    println!("  otpick add         Interactively add a normal SHA1/6-digit/30s TOTP");
    println!("  otpick import FILE Import otpauth:// TOTP URI lines from a file");
    println!("  otpick import -    Import otpauth:// TOTP URI lines from stdin");
    println!("  otpick --help      Show this help");
    println!("  otpick --version   Show the version");
}
