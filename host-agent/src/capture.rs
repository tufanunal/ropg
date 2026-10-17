use crate::grid::{SCREEN_WIDTH, SCREEN_HEIGHT};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{get_image, ImageFormat};
use x11rb::rust_connection::RustConnection;

pub struct ScreenCapture {
    frame_counter: u64,
    x11_conn: Option<(RustConnection, u32)>, // (connection, root_window)
    #[allow(dead_code)]
    is_real_display: bool,
}

impl ScreenCapture {
    pub fn new() -> Self {
        // Attempt to connect to real X11 Display
        let (conn_opt, is_real) = match x11rb::connect(None) {
            Ok((conn, screen_num)) => {
                let root = conn.setup().roots[screen_num].root;
                println!("[ScreenCapture] Successfully connected to X11 Display (root window: {})", root);
                (Some((conn, root)), true)
            }
            Err(e) => {
                println!("[ScreenCapture] Could not connect to X11 ({}); falling back to synthetic generator", e);
                (None, false)
            }
        };

        Self {
            frame_counter: 0,
            x11_conn: conn_opt,
            is_real_display: is_real,
        }
    }

    /// Captures a 1920x1080 RGBA buffer (hardware cursor excluded)
    pub fn capture_frame(&mut self) -> Vec<u8> {
        self.frame_counter += 1;

        if let Some((ref conn, root)) = self.x11_conn {
            match get_image(
                conn,
                ImageFormat::Z_PIXMAP,
                root,
                0,
                0,
                SCREEN_WIDTH as u16,
                SCREEN_HEIGHT as u16,
                !0,
            ) {
                Ok(cookie) => match cookie.reply() {
                    Ok(reply) => {
                        let raw_bytes = reply.data;
                        let mut rgba = vec![0u8; SCREEN_WIDTH * SCREEN_HEIGHT * 4];

                        // Convert X11 ZPixmap (typically B, G, R, [pad]) to RGBA (R, G, B, 255)
                        let max_pixels = (raw_bytes.len() / 4).min(SCREEN_WIDTH * SCREEN_HEIGHT);
                        for i in 0..max_pixels {
                            let src_idx = i * 4;
                            let dst_idx = i * 4;
                            rgba[dst_idx] = raw_bytes[src_idx + 2];     // Red
                            rgba[dst_idx + 1] = raw_bytes[src_idx + 1]; // Green
                            rgba[dst_idx + 2] = raw_bytes[src_idx];     // Blue
                            rgba[dst_idx + 3] = 255;                    // Alpha
                        }
                        return rgba;
                    }
                    Err(err) => {
                        eprintln!("[ScreenCapture] XGetImage reply error: {}", err);
                    }
                },
                Err(err) => {
                    eprintln!("[ScreenCapture] XGetImage error: {}", err);
                }
            }
        }

        // Fallback to simulated display
        self.generate_synthetic_frame()
    }

    fn generate_synthetic_frame(&mut self) -> Vec<u8> {
        let mut buffer = vec![0u8; SCREEN_WIDTH * SCREEN_HEIGHT * 4];
        let t = self.frame_counter;
        let anim_x = ((t * 8) % (SCREEN_WIDTH as u64 - 200)) as usize;
        let anim_y = 300 + (((t * 4) as f32).sin() * 100.0) as usize;

        for y in 0..SCREEN_HEIGHT {
            let row_offset = y * SCREEN_WIDTH * 4;
            let bg_b = 30 + ((y * 40) / SCREEN_HEIGHT) as u8;
            let bg_r = 15;
            let bg_g = 20;

            for x in 0..SCREEN_WIDTH {
                let px = row_offset + x * 4;

                buffer[px] = bg_r;
                buffer[px + 1] = bg_g;
                buffer[px + 2] = bg_b;
                buffer[px + 3] = 255;

                // Top Menu Bar
                if y < 36 {
                    buffer[px] = 30;
                    buffer[px + 1] = 41;
                    buffer[px + 2] = 59;
                }

                // Bottom Taskbar
                if y >= SCREEN_HEIGHT - 48 {
                    buffer[px] = 15;
                    buffer[px + 1] = 23;
                    buffer[px + 2] = 42;
                }

                // Moving Window Box
                if x >= anim_x && x < anim_x + 180 && y >= anim_y && y < anim_y + 120 {
                    if y < anim_y + 24 {
                        buffer[px] = 2;
                        buffer[px + 1] = 132;
                        buffer[px + 2] = 199;
                    } else {
                        buffer[px] = 51;
                        buffer[px + 1] = 65;
                        buffer[px + 2] = 85;
                    }
                }
            }
        }

        buffer
    }
}
