// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // The NSIS hooks run this binary to release the installation before copying.
    if let Some(code) = biank_desktop_lib::install_guard::from_args() {
        std::process::exit(code);
    }
    biank_desktop_lib::run();
}
