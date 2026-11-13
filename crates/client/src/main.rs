use std::process::exit;

use anyhow::Result;
use tracing::info;
use tracing_appender::non_blocking::WorkerGuard;

mod alert_service;
mod alerts;
mod api_client;
mod app;
mod body_finds;
mod build_info;
mod carrier_log;
mod cartography;
mod cli;
mod diagnostics;
mod eddn;
mod game_graphics;
mod ground_detail;
mod ground_tint;
mod engineering_data;
mod event_shim;
mod gui;
mod journal_follow;
mod journal_reader;
mod my_carriers;
mod pins;
mod planet_clouds;
mod planet_dust;
mod settings;
mod shader_mods;
mod star_light;
mod theme;
mod todo;
mod tts;
mod views;
mod wake;


fn main() -> Result<()> {
    let _file_guard = init_file_logging();

    info!("EDCAS client starting (build {})", build_info::BUILD);

    let args: Vec<String> = std::env::args().collect();
    info!("CLI arguments: {:?}", args);

    for arg in &args {
        match arg.as_str() {
            "--help" => {
                println!("EDCAS client — Elite Dangerous Commander Assistant System");
                println!("build {}", build_info::BUILD);
                println!();
                println!("Options:");
                println!("  --upload-logs   Upload all journal log files to the configured API server and exit.");
                println!("  --headless [--journal-dir DIR]");
                println!("                  No UI: only read the journal and upload to EDDN / the edcas API");
                println!("                  (as configured in the Settings tab) until stopped.");
                println!("  --version       Print the build number and exit.");
                println!("  --help          Show this help message.");
                println!();
                println!(
                    "Run `edcas-server` (separate binary) to start the EDDN listener and API."
                );
                exit(0);
            }
            "--version" | "-V" => {
                println!("edcas-client {}", build_info::BUILD);
                exit(0);
            }
            "--upload-logs" => {
                cli::upload_logs();
                exit(0);
            }
            "--headless" => {
                let journal_dir = args
                    .iter()
                    .position(|a| a == "--journal-dir")
                    .and_then(|i| args.get(i + 1))
                    .map(std::path::PathBuf::from);
                return cli::run_headless(journal_dir);
            }
            _ => {}
        }
    }

    #[cfg(windows)]
    hide_own_console();
    gui::run()
}

/// The binary is a console program (so `--help` and `--headless` work in a terminal),
/// which makes Windows open a console window when it is started by double-click. Close that
/// window when nobody else shares it; a console the user launched us from stays.
#[cfg(windows)]
fn hide_own_console() {
    use windows_sys::Win32::System::Console::{FreeConsole, GetConsoleProcessList};
    let mut pids = [0u32; 2];
    // SAFETY: the buffer is valid for `pids.len()` entries.
    let attached = unsafe { GetConsoleProcessList(pids.as_mut_ptr(), pids.len() as u32) };
    if attached == 1 {
        // SAFETY: detaching from our own console has no preconditions.
        unsafe { FreeConsole() };
    }
}

fn init_file_logging() -> WorkerGuard {
    // log.log in the per-user log folder (settings::log_dir); the last run's is kept as
    // log.previous.log, so a crash's log survives the restart. The temp directory if that
    // folder can't be written.
    let dir = settings::log_dir();
    let path = dir.join("log.log");
    let opened = std::fs::create_dir_all(&dir).and_then(|()| {
        if path.exists() {
            let _ = std::fs::rename(&path, dir.join("log.previous.log"));
        }
        std::fs::File::create(&path)
    });
    let (file, path) = match opened {
        Ok(file) => (file, path),
        Err(_) => {
            let fallback = std::env::temp_dir().join("edcas-client.log");
            (std::fs::File::create(&fallback).expect("Failed to create log file"), fallback)
        }
    };
    settings::set_log_file(path);
    let (non_blocking, guard) = tracing_appender::non_blocking(file);

    use tracing_subscriber::prelude::*;

    let default_filter = "warn,edcas_client=debug";
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(default_filter));

    let file_log = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_target(true)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .with_filter(filter);
    let subscriber = tracing_subscriber::registry().with(file_log);

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    guard
}
