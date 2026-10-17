use serde::Deserialize;
use std::collections::HashSet;
use x11rb::connection::Connection;
use x11rb::protocol::xtest::fake_input;
use x11rb::rust_connection::RustConnection;

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct InputCommand {
    #[serde(rename = "seqId")]
    pub seq_id: Option<u64>,
    #[serde(rename = "type")]
    pub event_type: String,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub button: Option<i32>,
    #[serde(rename = "deltaX")]
    pub delta_x: Option<i32>,
    #[serde(rename = "deltaY")]
    pub delta_y: Option<i32>,
    pub key: Option<String>,
    pub code: Option<String>,
}

pub struct InputDispatcher {
    processed_seq_ids: HashSet<u64>,
    max_history: usize,
    x11_conn: Option<(RustConnection, u32)>, // (connection, root_window)
}

impl InputDispatcher {
    pub fn new() -> Self {
        let conn_opt = match x11rb::connect(None) {
            Ok((conn, screen_num)) => {
                let root = conn.setup().roots[screen_num].root;
                Some((conn, root))
            }
            Err(_) => None,
        };

        Self {
            processed_seq_ids: HashSet::with_capacity(1000),
            max_history: 2000,
            x11_conn: conn_opt,
        }
    }

    pub fn dispatch(&mut self, cmd: &InputCommand) -> bool {
        // Sequence deduplication
        if let Some(seq) = cmd.seq_id {
            if self.processed_seq_ids.contains(&seq) {
                return false;
            }
            if self.processed_seq_ids.len() >= self.max_history {
                self.processed_seq_ids.clear();
            }
            self.processed_seq_ids.insert(seq);
        }

        match cmd.event_type.as_str() {
            "mouse_down" => {
                let x = cmd.x.unwrap_or(0);
                let y = cmd.y.unwrap_or(0);
                let btn = match cmd.button.unwrap_or(0) {
                    0 => 1, // Left click
                    1 => 2, // Middle click
                    2 => 3, // Right click
                    b => (b + 1) as u8,
                };
                self.x11_mouse_move(x, y);
                self.x11_mouse_button(btn, true);
                println!("[Input] Mouse Down button={} at ({}, {}) (seq={:?})", btn, x, y, cmd.seq_id);
            }
            "mouse_up" => {
                let x = cmd.x.unwrap_or(0);
                let y = cmd.y.unwrap_or(0);
                let btn = match cmd.button.unwrap_or(0) {
                    0 => 1,
                    1 => 2,
                    2 => 3,
                    b => (b + 1) as u8,
                };
                self.x11_mouse_move(x, y);
                self.x11_mouse_button(btn, false);
                println!("[Input] Mouse Up button={} at ({}, {}) (seq={:?})", btn, x, y, cmd.seq_id);
            }
            "mouse_drag" => {
                let x = cmd.x.unwrap_or(0);
                let y = cmd.y.unwrap_or(0);
                self.x11_mouse_move(x, y);
                println!("[Input] Mouse Drag to ({}, {})", x, y);
            }
            "mouse_hover" => {
                let x = cmd.x.unwrap_or(0);
                let y = cmd.y.unwrap_or(0);
                self.x11_mouse_move(x, y);
                println!("[Input] Mouse Hover at ({}, {})", x, y);
            }
            "mouse_wheel" => {
                let dy = cmd.delta_y.unwrap_or(0);
                let btn = if dy < 0 { 4 } else { 5 }; // 4: Scroll Up, 5: Scroll Down
                self.x11_mouse_button(btn, true);
                self.x11_mouse_button(btn, false);
                println!("[Input] Mouse Scroll dy={} (btn={})", dy, btn);
            }
            "key_down" => {
                let key = cmd.key.as_deref().unwrap_or("?");
                println!("[Input] Key Down: {} (code={:?})", key, cmd.code);
            }
            "key_up" => {
                let key = cmd.key.as_deref().unwrap_or("?");
                println!("[Input] Key Up: {}", key);
            }
            other => {
                println!("[Input] Unknown event type: {}", other);
            }
        }

        true
    }

    fn x11_mouse_move(&self, x: i32, y: i32) {
        if let Some((ref conn, root)) = self.x11_conn {
            // X11 MotionNotify event type = 6
            let _ = fake_input(
                conn,
                6,
                0,
                0,
                root,
                x as i16,
                y as i16,
                0,
            );
            let _ = conn.flush();
        }
    }

    fn x11_mouse_button(&self, button: u8, press: bool) {
        if let Some((ref conn, root)) = self.x11_conn {
            // X11 ButtonPress = 4, ButtonRelease = 5
            let event_type = if press { 4 } else { 5 };
            let _ = fake_input(conn, event_type, button, 0, root, 0, 0, 0);
            let _ = conn.flush();
        }
    }
}
