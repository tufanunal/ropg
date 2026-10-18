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

    // Force winit X11 scale factor to 1.0 so 1600x900 is strictly physical pixels
    std::env::set_var("WINIT_X11_SCALE_FACTOR", "1.0");

    // Load ROGP application icon
    let icon_data = load_app_icon();

    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title("ROGP Host Agent v1.2.0 - FHD Grid Diffing Engine")
        .with_inner_size([1600.0, 900.0])
        .with_min_inner_size([1600.0, 900.0])
        .with_max_inner_size([1600.0, 900.0])
        .with_resizable(false);

    if let Some(icon) = icon_data {
        viewport = viewport.with_icon(icon);
    }

    // Launch Native GUI
    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "ROGP Host Agent v1.2.0 - FHD Grid Diffing Engine",
        native_options,
        Box::new(|cc| Ok(Box::new(RogpHostApp::new(cc, state)))),
    )
}

fn load_app_icon() -> Option<eframe::egui::IconData> {
    let icon_bytes = include_bytes!("assets/icon.png");
    let img = image::load_from_memory(icon_bytes).ok()?.to_rgba8();
    let (width, height) = img.dimensions();
    Some(eframe::egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    })
}
