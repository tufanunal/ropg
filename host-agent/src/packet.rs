use byteorder::{BigEndian, WriteBytesExt};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

pub const BINARY_MAGIC: u32 = 0x524F4750; // "ROGP"
pub const PROTOCOL_VERSION: u8 = 1;

#[derive(Serialize)]
pub struct JsonCellItem {
    pub id: String,
    pub v: u32,
    pub data: String, // base64 encoded WebP
}

#[derive(Serialize)]
pub struct JsonPayload {
    pub host_id: String,
    pub full_sync: bool,
    pub timestamp: u64,
    pub cells: Vec<JsonCellItem>,
}

pub struct EncodedCell {
    pub row: u8,
    pub col: u8,
    pub cell_id: String,
    pub version: u32,
    pub webp_data: Vec<u8>,
}

/// Builds Mode A JSON payload string
pub fn build_json_packet(host_id: &str, is_full_sync: bool, cells: &[EncodedCell]) -> String {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let json_cells: Vec<JsonCellItem> = cells
        .iter()
        .map(|c| JsonCellItem {
            id: c.cell_id.clone(),
            v: c.version,
            data: BASE64_STANDARD.encode(&c.webp_data),
        })
        .collect();

    let payload = JsonPayload {
        host_id: host_id.to_string(),
        full_sync: is_full_sync,
        timestamp: now_ms,
        cells: json_cells,
    };

    serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string())
}

/// Builds Mode B Binary packet
pub fn build_binary_packet(is_full_sync: bool, is_grayscale: bool, cells: &[EncodedCell]) -> Vec<u8> {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let mut buf = Vec::with_capacity(16 + cells.len() * 100);

    // [0..3] Magic 0x524F4750
    buf.write_u32::<BigEndian>(BINARY_MAGIC).unwrap();
    // [4] Version 1
    buf.write_u8(PROTOCOL_VERSION).unwrap();

    // [5] Flags
    let mut flags = 0u8;
    if is_full_sync {
        flags |= 0x01;
    }
    if is_grayscale {
        flags |= 0x02;
    }
    buf.write_u8(flags).unwrap();

    // [6..7] Cell Count
    buf.write_u16::<BigEndian>(cells.len() as u16).unwrap();

    // [8..15] Timestamp
    buf.write_u64::<BigEndian>(now_ms).unwrap();

    // Each cell
    for cell in cells {
        buf.write_u8(cell.row).unwrap();
        buf.write_u8(cell.col).unwrap();
        buf.write_u32::<BigEndian>(cell.version).unwrap();
        buf.write_u32::<BigEndian>(cell.webp_data.len() as u32).unwrap();
        buf.extend_from_slice(&cell.webp_data);
    }

    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use byteorder::{BigEndian, ReadBytesExt};
    use std::io::Cursor;

    #[test]
    fn test_binary_packet_structure() {
        let cells = vec![
            EncodedCell {
                row: 3,
                col: 16,
                cell_id: "C17".to_string(),
                version: 42,
                webp_data: vec![1, 2, 3, 4],
            }
        ];

        let bytes = build_binary_packet(true, false, &cells);
        assert!(bytes.len() >= 16);

        let mut r = Cursor::new(&bytes);
        let magic = r.read_u32::<BigEndian>().unwrap();
        assert_eq!(magic, BINARY_MAGIC);

        let version = r.read_u8().unwrap();
        assert_eq!(version, PROTOCOL_VERSION);

        let flags = r.read_u8().unwrap();
        assert_eq!(flags & 0x01, 0x01); // full sync bit set

        let cell_count = r.read_u16::<BigEndian>().unwrap();
        assert_eq!(cell_count, 1);

        let _timestamp = r.read_u64::<BigEndian>().unwrap();

        // Cell 0
        let row = r.read_u8().unwrap();
        assert_eq!(row, 3);
        let col = r.read_u8().unwrap();
        assert_eq!(col, 16);
        let cell_ver = r.read_u32::<BigEndian>().unwrap();
        assert_eq!(cell_ver, 42);
        let data_len = r.read_u32::<BigEndian>().unwrap();
        assert_eq!(data_len, 4);
    }

    #[test]
    fn test_json_packet_structure() {
        let cells = vec![
            EncodedCell {
                row: 0,
                col: 0,
                cell_id: "001".to_string(),
                version: 1,
                webp_data: vec![10, 20],
            }
        ];

        let json_str = build_json_packet("host-test", false, &cells);
        assert!(json_str.contains("\"host_id\":\"host-test\""));
        assert!(json_str.contains("\"id\":\"001\""));
        assert!(json_str.contains("\"v\":1"));
    }
}
