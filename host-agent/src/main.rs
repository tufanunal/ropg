mod grid;
mod encoder;
mod packet;
mod input;
mod capture;
mod client;
mod state;
mod worker;
mod ui;

use clap::Parser;
use state::HostSharedState;
use worker::start_worker_thread;
use ui::RogpHostApp;

#[derive(Parser, Debug)]
#[command(name = "Rogp Host Agent")]
#[command(about = "HTTP-based Host Agent with Grid Diffing and WebP encoding for Debian 12")]
struct Args {
    #[arg(long, default_value = "http://127.0.0.1:8080")]
    relay_url: String,

    #[arg(long, default_value = "host-1")]
    host_id: String,

    #[arg(long, default_value = "rogp-secret-token")]
    token: String,

    #[arg(long, default_value = "binary")]
    mode: String, // "binary" or "json"

    #[arg(long, default_value_t = 80)]
    quality: u8,

    #[arg(long, default_value_t = false)]
    grayscale: bool,

    #[arg(long, default_value_t = 200)]
    interval_ms: u64,

    #[arg(long, default_value_t = false)]
    headless: bool,
}

fn main() -> eframe::Result {
    let args = Args::parse();

    println!("=======================================================");
    println!("  Rogp Host Agent (Debian 12 Native Engine)");
    println!("  Target Resolution : 1920x1080 FHD");
    println!("  Grid Dimensions   : 48 Cols x 27 Rows (1,296 Cells)");
    println!("  Relay Server      : {}", args.relay_url);
    println!("  Host Identifier   : {}", args.host_id);
    println!("  Payload Mode      : {}", args.mode);
    println!("  GUI Enabled       : {}", !args.headless);
    println!("=======================================================\n");

    let is_binary_mode = args.mode.to_lowercase() == "binary";
    let state = HostSharedState::new(
        args.relay_url,
        args.host_id,
        args.token,
        is_binary_mode,
        args.quality,
        args.grayscale,
        args.interval_ms,
        !args.headless,
    );

    // Spawn the background screen capture & network worker thread
    start_worker_thread(state.clone());

    if args.headless {
        println!("[Rogp] Running in Headless mode (CLI). Press Ctrl+C to terminate.");
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }

    // Launch Native Linux (Debian 12) GUI
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("ROGP Host Agent v1.2.0 - FHD Grid Diffing Engine")
            .with_inner_size([1100.0, 780.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "ROGP Host Agent v1.2.0 - FHD Grid Diffing Engine",
        native_options,
        Box::new(|cc| Ok(Box::new(RogpHostApp::new(cc, state)))),
    )
}
