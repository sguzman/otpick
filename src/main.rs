mod app;
mod backup;
mod clipboard;
mod desktop_keyring;
mod ingest;
mod model;
mod paths;
mod qr;
mod search;
mod session;
mod startup;
mod totp;
mod vault;

use std::error::Error;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use app::PickerApp;
use eframe::egui;
use model::{Account, display_labels};
use vault::Vault;
use zeroize::Zeroizing;

fn main() {
    let startup_trace = startup::StartupTrace::from_env();
    startup_trace.mark("process-entry");

    if let Err(error) = run(&startup_trace) {
        eprintln!("otpick: {error}");
        std::process::exit(1);
    }
}

fn run(startup_trace: &startup::StartupTrace) -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);

    match args.next().as_deref() {
        Some("-h" | "--help" | "help") => print_help(),
        Some("-V" | "--version") => println!("otpick {}", env!("CARGO_PKG_VERSION")),
        Some("init") => init_vault()?,
        Some("unlock") => unlock_vault()?,
        Some("lock") => lock_vault()?,
        Some("keyring") => match args.next().as_deref() {
            Some("enable") => {
                ensure_no_extra_args(&mut args, "keyring enable")?;
                enable_desktop_keyring()?;
            }
            Some("disable") => {
                ensure_no_extra_args(&mut args, "keyring disable")?;
                disable_desktop_keyring()?;
            }
            Some(other) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown keyring command: {other}"),
                )
                .into());
            }
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "keyring requires enable or disable",
                )
                .into());
            }
        },
        Some("status") => status()?,
        Some("list") => list_accounts()?,
        Some("add") => add_account()?,
        Some("backup") => {
            let directory = args.next().map(PathBuf::from);
            if args.next().is_some() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "backup accepts at most one destination directory",
                )
                .into());
            }
            backup_vault(directory.as_deref())?;
        }
        Some("import") => {
            let sources: Vec<String> = args.collect();
            if sources.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "import requires at least one file path or - for stdin",
                )
                .into());
            }
            import_accounts(&sources)?;
        }
        Some(other) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unknown command: {other}"),
            )
            .into());
        }
        None => run_picker(startup_trace.clone())?,
    }

    Ok(())
}

fn run_picker(startup_trace: startup::StartupTrace) -> eframe::Result {
    startup_trace.mark("picker-entry");
    let (accounts, notice) = load_picker_accounts(&startup_trace);
    startup_trace.mark("accounts-loaded");

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

    startup_trace.mark("run-native-enter");
    eframe::run_native(
        "OTPick",
        options,
        Box::new(move |cc| {
            Ok(Box::new(PickerApp::new(
                cc,
                accounts,
                notice,
                startup_trace.clone(),
            )))
        }),
    )
}

fn load_picker_accounts(startup_trace: &startup::StartupTrace) -> (Vec<Account>, Option<String>) {
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

    startup_trace.mark("vault-path-ready");
    let key = match load_vault_key(&path) {
        Ok(Some(key)) => key,
        Ok(None) => {
            return (
                Vec::new(),
                Some(
                    "Vault locked. Run otpick unlock, or enable desktop keyring integration once."
                        .to_owned(),
                ),
            );
        }
        Err(error) => return (Vec::new(), Some(error.to_string())),
    };

    startup_trace.mark("session-key-loaded");
    match Vault::open_with_key(&path, key) {
        Ok(vault) => {
            startup_trace.mark("vault-decrypted");
            (vault.into_accounts(), None)
        }
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
    session::clear_manual_lock()?;

    if let Some(key) = desktop_keyring::load(&path)? {
        match Vault::open_with_key(&path, key) {
            Ok(vault) => {
                session::store(vault.key())?;
                println!("Unlocked from desktop keyring for this login session.");
                return Ok(());
            }
            Err(_) => {
                session::clear()?;
            }
        }
    }

    let passphrase = Zeroizing::new(rpassword::prompt_password("OTPick passphrase: ")?);
    let vault = Vault::unlock(&path, passphrase.as_bytes())?;
    session::store(vault.key())?;
    println!("Unlocked for this login session.");
    Ok(())
}

fn lock_vault() -> Result<(), Box<dyn Error>> {
    let was_unlocked = session::clear()?;
    session::mark_locked()?;

    if was_unlocked {
        println!("Locked for this login session.");
    } else {
        println!("Locked for this login session.");
    }
    Ok(())
}

fn enable_desktop_keyring() -> Result<(), Box<dyn Error>> {
    let path = paths::vault_path()?;

    let vault = match session::load()? {
        Some(key) => Vault::open_with_key(&path, key)?,
        None => {
            let passphrase = Zeroizing::new(rpassword::prompt_password("OTPick passphrase: ")?);
            let vault = Vault::unlock(&path, passphrase.as_bytes())?;
            session::store(vault.key())?;
            vault
        }
    };

    desktop_keyring::store(&path, vault.key())?;
    println!("Desktop keyring integration enabled.");
    Ok(())
}

fn disable_desktop_keyring() -> Result<(), Box<dyn Error>> {
    let path = paths::vault_path()?;
    if desktop_keyring::remove(&path)? {
        println!("Desktop keyring integration disabled.");
    } else {
        println!("Desktop keyring integration was not enabled.");
    }
    Ok(())
}

fn status() -> Result<(), Box<dyn Error>> {
    let path = paths::vault_path()?;
    println!("Vault: {}", path.display());
    println!("Exists: {}", path.exists());
    println!("Session unlocked: {}", session::load()?.is_some());
    println!("Manual session lock: {}", session::is_manually_locked()?);

    match desktop_keyring::exists(&path) {
        Ok(enabled) => println!("Desktop keyring enabled: {enabled}"),
        Err(error) => println!("Desktop keyring: unavailable ({error})"),
    }

    Ok(())
}

fn list_accounts() -> Result<(), Box<dyn Error>> {
    let (_, vault) = open_unlocked_vault()?;

    for label in display_labels(vault.accounts()) {
        println!("{label}");
    }

    Ok(())
}

fn add_account() -> Result<(), Box<dyn Error>> {
    let issuer = prompt_line("Issuer: ")?;
    let account_name = prompt_line("Account: ")?;
    let secret = Zeroizing::new(rpassword::prompt_password("Base32 TOTP secret: ")?);

    let account = ingest::account_from_base32(issuer, account_name, secret.as_str())?;
    if persist_accounts(vec![account])? == 0 {
        println!("Account already exists.");
    } else {
        println!("Added account.");
    }
    Ok(())
}

fn backup_vault(destination: Option<&Path>) -> Result<(), Box<dyn Error>> {
    let (vault_path, _) = open_unlocked_vault()?;
    let backup_dir = match destination {
        Some(path) => path.to_path_buf(),
        None => paths::backup_dir()?,
    };

    let backup_path = backup::create(&vault_path, &backup_dir)?;
    println!("Backup created: {}", backup_path.display());
    Ok(())
}

fn import_accounts(sources: &[String]) -> Result<(), Box<dyn Error>> {
    if sources.len() > 1 && sources.iter().any(|source| source == "-") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "stdin (-) cannot be combined with other import sources",
        )
        .into());
    }

    let mut input = Zeroizing::new(String::new());

    for source in sources {
        if !input.is_empty() && !input.ends_with('\n') {
            input.push('\n');
        }

        if source == "-" {
            io::stdin().read_to_string(&mut input)?;
            continue;
        }

        let path = Path::new(source);
        if is_image_source(path) {
            let payloads = qr::decode_file(path)?;
            for payload in payloads {
                input.push_str(payload.as_str());
                input.push('\n');
            }
        } else {
            File::open(path)?.read_to_string(&mut input)?;
        }
    }

    let accounts = ingest::parse_document(input.as_str())?;
    let count = persist_accounts(accounts)?;

    println!("Imported {count} account(s).");
    Ok(())
}

fn is_image_source(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "webp"
            )
        })
}

fn persist_accounts(accounts: Vec<Account>) -> Result<usize, Box<dyn Error>> {
    let (path, mut vault) = open_unlocked_vault()?;
    let accounts = ingest::deduplicate_accounts(vault.accounts(), accounts);
    let count = accounts.len();

    if count > 0 {
        vault.accounts_mut().extend(accounts);
        vault.save(&path)?;
    }

    Ok(count)
}

fn open_unlocked_vault() -> Result<(PathBuf, Vault), Box<dyn Error>> {
    let path = paths::vault_path()?;
    let key = load_vault_key(&path)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            "vault is locked; run otpick unlock",
        )
    })?;
    let vault = Vault::open_with_key(&path, key)?;
    Ok((path, vault))
}

fn load_vault_key(path: &Path) -> Result<Option<vault::VaultKey>, Box<dyn Error>> {
    if let Some(key) = session::load()? {
        return Ok(Some(key));
    }

    if session::is_manually_locked()? {
        return Ok(None);
    }

    let Some(key) = desktop_keyring::load(path)? else {
        return Ok(None);
    };

    session::store(&key)?;
    Ok(Some(key))
}

fn ensure_no_extra_args(
    args: &mut impl Iterator<Item = String>,
    command: &str,
) -> Result<(), Box<dyn Error>> {
    if args.next().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{command} accepts no additional arguments"),
        )
        .into());
    }

    Ok(())
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
    println!("  otpick lock        Lock OTPick for this login session");
    println!("  otpick keyring enable  Store the vault key in desktop Secret Service");
    println!("  otpick keyring disable Remove the persistent desktop-keyring copy");
    println!("  otpick status      Show vault, session, and desktop-keyring state");
    println!("  otpick list        List account labels without exposing codes");
    println!("  otpick add         Interactively add a normal SHA1/6-digit/30s TOTP");
    println!("  otpick backup [DIR] Snapshot the encrypted vault");
    println!("  otpick import SRC... Import text or QR image sources (PNG/JPEG/WebP)");
    println!("  otpick import -      Import OTP URI lines from stdin");
    println!("  otpick --help      Show this help");
    println!("  otpick --version   Show the version");
}
