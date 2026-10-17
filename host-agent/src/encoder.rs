use std::io::Cursor;
use image::{ColorType, ImageEncoder};
use image::codecs::webp::WebPEncoder;
use crate::grid::{CELL_WIDTH, CELL_HEIGHT};

pub struct CellEncoder {
    #[allow(dead_code)]
    pub quality: u8,
    pub grayscale: bool,
}

impl CellEncoder {
    pub fn new(quality: u8, grayscale: bool) -> Self {
        Self { quality, grayscale }
    }

    /// Encodes a 40x40 pixel buffer into WebP bytes
    pub fn encode_cell(&self, rgba: &[u8]) -> Result<Vec<u8>, String> {
        let mut output = Vec::new();

        if self.grayscale {
            // Convert RGBA to 8-bit Grayscale: Y = 0.299R + 0.587G + 0.114B
            let mut gray = Vec::with_capacity(CELL_WIDTH * CELL_HEIGHT);
            for chunk in rgba.chunks_exact(4) {
                let r = chunk[0] as f32;
                let g = chunk[1] as f32;
                let b = chunk[2] as f32;
                let y = (0.299 * r + 0.587 * g + 0.114 * b) as u8;
                gray.push(y);
            }

            let mut cursor = Cursor::new(&mut output);
            let encoder = WebPEncoder::new_lossless(&mut cursor);
            encoder.write_image(&gray, CELL_WIDTH as u32, CELL_HEIGHT as u32, ColorType::L8.into())
                .map_err(|e| format!("WebP grayscale encode error: {}", e))?;
        } else {
            let mut cursor = Cursor::new(&mut output);
            let encoder = WebPEncoder::new_lossless(&mut cursor);
            encoder.write_image(rgba, CELL_WIDTH as u32, CELL_HEIGHT as u32, ColorType::Rgba8.into())
                .map_err(|e| format!("WebP RGBA encode error: {}", e))?;
        }

        Ok(output)
    }
}
