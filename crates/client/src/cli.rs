use std::path::PathBuf;

use std::time::Duration;

use tracing::{info, warn};

use crate::journal_reader::{start_bulk_upload, JournalReader};
use crate::settings::Settings;

/// Uploads all journal log files to the configured API server, printing progress
/// to stdout. Exits with code 1 on error, 0 on success.
pub fn upload_logs() {
    let settings = Settings::default();

    let Some(api_url) = settings.edcas_api_url() else {
        if !settings.edcas_api_enabled {
            eprintln!("Error: edcas API upload is disabled. Enable it in the Settings tab first.");
        } else {
            eprintln!("Error: no API URL configured. Set it in the Settings tab first.");
        }
        std::process::exit(1);
    };

    let journal_dir = PathBuf::from(&settings.journal_reader.journal_directory);
    if !journal_dir.exists() {
        eprintln!(
            "Error: journal directory does not exist: {}",
            journal_dir.display()
        );
        std::process::exit(1);
    }

    println!("Uploading journal logs from: {}", journal_dir.display());
    println!("Server: {}", api_url);

    let rx = start_bulk_upload(journal_dir, api_url);

    for progress in rx {
        if progress.total_files > 0 {
            print!(
                "\rFile {}/{} — {} events uploaded",
                progress.current_file, progress.total_files, progress.lines_done
            );
            if let Some(ref err) = progress.error {
                print!(" [{}]", err);
            }
            use std::io::Write;
            let _ = std::io::stdout().flush();
        }
        if progress.done {
            println!();
            if let Some(err) = progress.error {
                eprintln!("Upload finished with errors: {}", err);
                std::process::exit(1);
            } else {
                println!("Upload complete.");
            }
            return;
        }
    }

    eprintln!("Upload thread ended unexpectedly.");
    std::process::exit(1);
}

/// Runs only the journal reader and its uploaders (EDDN, edcas API) without the
/// window, until SIGINT/SIGTERM. For running next to an unattended game.
pub fn run_headless(journal_dir: Option<PathBuf>) -> anyhow::Result<()> {
    let settings = Settings::default();
    let journal_dir =
        journal_dir.unwrap_or_else(|| PathBuf::from(&settings.journal_reader.journal_directory));
    if !journal_dir.is_dir() {
        anyhow::bail!(
            "journal directory does not exist: {}",
            journal_dir.display()
        );
    }
    let api_url = settings.edcas_api_url();
    let eddn = settings.eddn_config();
    info!(
        "headless: journal {}, edcas API {:?}, EDDN {}",
        journal_dir.display(),
        api_url,
        eddn.as_ref().map_or("off".to_string(), |c| c.url.clone())
    );
    if api_url.is_none() && eddn.is_none() {
        warn!("headless: neither EDDN nor edcas API uploads are enabled — nothing to do");
    }
    println!(
        "edcas-client headless: reading {} (EDDN {}, edcas API {})",
        journal_dir.display(),
        if eddn.is_some() { "on" } else { "off" },
        api_url.as_deref().unwrap_or("off")
    );

    let mut reader = JournalReader::start(journal_dir, api_url, eddn);
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let mut tick = tokio::time::interval(Duration::from_millis(500));
        let stop = shutdown_signal();
        tokio::pin!(stop);
        loop {
            tokio::select! {
                _ = &mut stop => break,
                // The reader publishes a snapshot per change; nobody displays them.
                _ = tick.tick() => while reader.try_recv().is_some() {},
            }
        }
    });
    info!("headless: stopping");
    reader.stop();
    // Uploader threads drain their queues once the reader drops its senders.
    std::thread::sleep(Duration::from_secs(3));
    Ok(())
}

/// Resolves on Ctrl-C, or SIGTERM on Unix.
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        match signal(SignalKind::terminate()) {
            Ok(mut term) => {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {}
                    _ = term.recv() => {}
                }
            }
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
