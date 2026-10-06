mod app;
mod clipboard;
mod model;
mod paths;
mod search;
mod session;
mod totp;
mod vault;

use std::error::Error;
use std::io;

use app::PickerApp;
use eframe::egui;
use vault::Vault;
use zeroize::Zeroizing;

fn main() {
    if let Err(error) = run() {
        eprintln!("otpick: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    match std::env::args().nth(1).as_deref() {
        Some("-h" | "--help" | "help") => print_help(),
        Some("-V" | "--version") => println!("otpick {}", env!("CARGO_PKG_VERSION")),
        Some("init") => init_vault()?,
        Some("unlock") => unlock_vault()?,
        Some("lock") => lock_vault()?,
        Some("status") => status()?,
        Some("list") => list_accounts()?,
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

fn load_picker_accounts() -> (Vec<model::Account>, Option<String>) {
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
    let path = paths::vault_path()?;
    let key = session::load()?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            "vault is locked; run otpick unlock",
        )
    })?;
    let vault = Vault::open_with_key(&path, key)?;

    for account in vault.accounts() {
        println!("{}", account.label());
    }

    Ok(())
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
    println!("  otpick           Open the picker");
    println!("  otpick init      Create an encrypted vault and unlock it");
    println!("  otpick unlock    Unlock the vault for this login session");
    println!("  otpick lock      Remove the session key");
    println!("  otpick status    Show vault and session state");
    println!("  otpick list      List account labels without exposing codes");
    println!("  otpick --help    Show this help");
    println!("  otpick --version Show the version");
}
