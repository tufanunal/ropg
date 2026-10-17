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
                let code = cmd.code.as_deref().unwrap_or("");
                if let Some(kc) = dom_code_to_x11_keycode(code, cmd.key.as_deref()) {
                    self.x11_key(kc, true);
                    println!("[Input] Key Down: {} (code={:?}, keycode={})", key, cmd.code, kc);
                } else {
                    println!("[Input] Key Down (unmapped): {} (code={:?})", key, cmd.code);
                }
            }
            "key_up" => {
                let key = cmd.key.as_deref().unwrap_or("?");
                let code = cmd.code.as_deref().unwrap_or("");
                if let Some(kc) = dom_code_to_x11_keycode(code, cmd.key.as_deref()) {
                    self.x11_key(kc, false);
                    println!("[Input] Key Up: {} (code={:?}, keycode={})", key, cmd.code, kc);
                } else {
                    println!("[Input] Key Up (unmapped): {} (code={:?})", key, cmd.code);
                }
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

    fn x11_key(&self, keycode: u8, press: bool) {
        if let Some((ref conn, root)) = self.x11_conn {
            // X11 KeyPress = 2, KeyRelease = 3
            let event_type = if press { 2 } else { 3 };
            let _ = fake_input(conn, event_type, keycode, 0, root, 0, 0, 0);
            let _ = conn.flush();
        }
    }
}

/// Maps browser DOM Level 3 KeyboardEvent.code and key to standard Linux X11 Keycode (evdev + 8)
pub fn dom_code_to_x11_keycode(code: &str, key: Option<&str>) -> Option<u8> {
    match code {
        // Alphanumeric keys
        "Escape" => Some(9),
        "Digit1" => Some(10),
        "Digit2" => Some(11),
        "Digit3" => Some(12),
        "Digit4" => Some(13),
        "Digit5" => Some(14),
        "Digit6" => Some(15),
        "Digit7" => Some(16),
        "Digit8" => Some(17),
        "Digit9" => Some(18),
        "Digit0" => Some(19),
        "Minus" => Some(20),
        "Equal" => Some(21),
        "Backspace" => Some(22),
        "Tab" => Some(23),
        "KeyQ" => Some(24),
        "KeyW" => Some(25),
        "KeyE" => Some(26),
        "KeyR" => Some(27),
        "KeyT" => Some(28),
        "KeyY" => Some(29),
        "KeyU" => Some(30),
        "KeyI" => Some(31),
        "KeyO" => Some(32),
        "KeyP" => Some(33),
        "BracketLeft" => Some(34),
        "BracketRight" => Some(35),
        "Enter" => Some(36),
        "ControlLeft" => Some(37),
        "KeyA" => Some(38),
        "KeyS" => Some(39),
        "KeyD" => Some(40),
        "KeyF" => Some(41),
        "KeyG" => Some(42),
        "KeyH" => Some(43),
        "KeyJ" => Some(44),
        "KeyK" => Some(45),
        "KeyL" => Some(46),
        "Semicolon" => Some(47),
        "Quote" => Some(48),
        "Backquote" => Some(49),
        "ShiftLeft" => Some(50),
        "Backslash" => Some(51),
        "KeyZ" => Some(52),
        "KeyX" => Some(53),
        "KeyC" => Some(54),
        "KeyV" => Some(55),
        "KeyB" => Some(56),
        "KeyN" => Some(57),
        "KeyM" => Some(58),
        "Comma" => Some(59),
        "Period" => Some(60),
        "Slash" => Some(61),
        "ShiftRight" => Some(62),
        "NumpadMultiply" => Some(63),
        "AltLeft" => Some(64),
        "Space" => Some(65),
        "CapsLock" => Some(66),
        "F1" => Some(67),
        "F2" => Some(68),
        "F3" => Some(69),
        "F4" => Some(70),
        "F5" => Some(71),
        "F6" => Some(72),
        "F7" => Some(73),
        "F8" => Some(74),
        "F9" => Some(75),
        "F10" => Some(76),
        "NumLock" => Some(77),
        "ScrollLock" => Some(78),
        "Numpad7" => Some(79),
        "Numpad8" => Some(80),
        "Numpad9" => Some(81),
        "NumpadSubtract" => Some(82),
        "Numpad4" => Some(83),
        "Numpad5" => Some(84),
        "Numpad6" => Some(85),
        "NumpadAdd" => Some(86),
        "Numpad1" => Some(87),
        "Numpad2" => Some(88),
        "Numpad3" => Some(89),
        "Numpad0" => Some(90),
        "NumpadDecimal" => Some(91),
        "IntlBackslash" => Some(94),
        "F11" => Some(95),
        "F12" => Some(96),
        "NumpadEnter" => Some(104),
        "ControlRight" => Some(105),
        "NumpadDivide" => Some(106),
        "PrintScreen" => Some(107),
        "AltRight" => Some(108),
        "Home" => Some(110),
        "ArrowUp" => Some(111),
        "PageUp" => Some(112),
        "ArrowLeft" => Some(113),
        "ArrowRight" => Some(114),
        "End" => Some(115),
        "ArrowDown" => Some(116),
        "PageDown" => Some(117),
        "Insert" => Some(118),
        "Delete" => Some(119),
        "Pause" => Some(127),
        "MetaLeft" => Some(133),
        "MetaRight" => Some(134),
        "ContextMenu" => Some(135),
        _ => {
            // Fallback for single characters if code is missing or unmapped
            if let Some(k) = key {
                match k.to_lowercase().as_str() {
                    "a" => Some(38),
                    "b" => Some(56),
                    "c" => Some(54),
                    "d" => Some(40),
                    "e" => Some(26),
                    "f" => Some(41),
                    "g" => Some(42),
                    "h" => Some(43),
                    "i" => Some(31),
                    "j" => Some(44),
                    "k" => Some(45),
                    "l" => Some(46),
                    "m" => Some(58),
                    "n" => Some(57),
                    "o" => Some(32),
                    "p" => Some(33),
                    "q" => Some(24),
                    "r" => Some(27),
                    "s" => Some(39),
                    "t" => Some(28),
                    "u" => Some(30),
                    "v" => Some(55),
                    "w" => Some(25),
                    "x" => Some(53),
                    "y" => Some(29),
                    "z" => Some(52),
                    " " => Some(65),
                    "\n" | "\r" | "enter" => Some(36),
                    "backspace" => Some(22),
                    "tab" => Some(23),
                    "escape" => Some(9),
                    _ => None,
                }
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dom_code_mapping() {
        assert_eq!(dom_code_to_x11_keycode("KeyN", Some("N")), Some(57));
        assert_eq!(dom_code_to_x11_keycode("KeyO", Some("o")), Some(32));
        assert_eq!(dom_code_to_x11_keycode("KeyW", Some("w")), Some(25));
        assert_eq!(dom_code_to_x11_keycode("Space", Some(" ")), Some(65));
        assert_eq!(dom_code_to_x11_keycode("ShiftRight", Some("Shift")), Some(62));
        assert_eq!(dom_code_to_x11_keycode("ControlLeft", Some("Control")), Some(37));
        assert_eq!(dom_code_to_x11_keycode("Enter", Some("Enter")), Some(36));
        assert_eq!(dom_code_to_x11_keycode("Backspace", Some("Backspace")), Some(22));
    }
}
