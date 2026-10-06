mod app;
mod clipboard;
mod model;
mod search;
mod totp;

use app::PickerApp;
use eframe::egui;

fn main() -> eframe::Result {
    match std::env::args().nth(1).as_deref() {
        Some("-h" | "--help") => {
            print_help();
            return Ok(());
        }
        Some("-V" | "--version") => {
            println!("otpick {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        Some(other) => {
            eprintln!("otpick: unknown argument: {other}");
            print_help();
            std::process::exit(2);
        }
        None => {}
    }

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
        Box::new(|cc| Ok(Box::new(PickerApp::new(cc, Vec::new())))),
    )
}

fn print_help() {
    println!("OTPick - one-shot TOTP picker");
    println!();
    println!("Usage:");
    println!("  otpick           Open the picker");
    println!("  otpick --help    Show this help");
    println!("  otpick --version Show the version");
}
