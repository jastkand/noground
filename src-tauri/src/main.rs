// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // the Settings-installed symlink runs this binary as `noground` — see
    // cli.rs for the dispatch
    if noground_lib::cli::should_run_cli() {
        std::process::exit(noground_lib::cli::run());
    }
    noground_lib::run()
}
