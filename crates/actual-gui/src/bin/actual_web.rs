//! `actual-web` binary utility for PWA packaging, static asset distribution, and Web diagnostics.

use actual_gui::web::{IndexHtmlBuilder, ServiceWorkerBuilder, WebApp, WebManifest, WebSyncConfig};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "actual-web",
    about = "Actual Budget Web (WASM & OPFS) client manager"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Run non-interactive headless verification of OPFS and web state machine.
    #[arg(long)]
    headless_check: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Export static web assets (index.html, manifest.webmanifest, sw.js) to a directory.
    ExportDist {
        #[arg(short, long, default_value = "dist")]
        output: PathBuf,
    },
    /// Validate PWA manifest and service worker configuration.
    VerifyPwa,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if cli.headless_check {
        println!("Running actual-web self-diagnostic...");
        let mut app = WebApp::new("/actual-storage", WebSyncConfig::default());
        app.create_budget("test-budget", "Test Budget", 1700000000)?;
        app.save_active_budget(1700000001)?;
        println!(
            "OPFS Budget state verified successfully. Routes: {:?}, Quota usage: {} bytes",
            app.current_route,
            app.storage.estimate_quota().usage_bytes
        );
        return Ok(());
    }

    match cli.command {
        Some(Commands::ExportDist { output }) => {
            println!("Exporting web client assets to: {}", output.display());
            fs::create_dir_all(&output)?;

            // 1. Index HTML
            let html = IndexHtmlBuilder::generate_html("Actual Budget", "#2563eb");
            fs::write(output.join("index.html"), html)?;

            // 2. Web App Manifest
            let manifest = WebManifest::default();
            fs::write(output.join("manifest.webmanifest"), manifest.to_json()?)?;

            // 3. Service Worker
            let sw = ServiceWorkerBuilder::default().generate_script();
            fs::write(output.join("sw.js"), sw)?;

            println!(
                "Web distribution assets successfully written to {}",
                output.display()
            );
        }
        Some(Commands::VerifyPwa) => {
            let manifest = WebManifest::default();
            let json = manifest.to_json()?;
            assert!(json.contains("Actual Budget"));
            assert!(json.contains("standalone"));

            let sw = ServiceWorkerBuilder::default().generate_script();
            assert!(sw.contains("CACHE_NAME"));
            assert!(sw.contains("/sync/"));

            println!("PWA manifest and Service Worker verification PASSED.");
        }
        None => {
            println!("Actual Budget Web (WASM & OPFS) client. Run with --help for options.");
        }
    }

    Ok(())
}
