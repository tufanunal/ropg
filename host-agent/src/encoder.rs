use webp::{Encoder, WebPConfig};

pub struct CellEncoder {
    pub quality: u8,
    pub grayscale: bool,
    pub cell_width: usize,
    pub cell_height: usize,
    pub method: i32,
}

impl CellEncoder {
    pub fn new(quality: u8, grayscale: bool, cell_width: usize, cell_height: usize, method: i32) -> Self {
        Self {
            quality,
            grayscale,
            cell_width,
            cell_height,
            method: method.clamp(4, 6),
        }
    }

    /// Encodes a cell's pixel buffer into real lossy/quality WebP bytes with configurable compression method (4, 5, 6)
    pub fn encode_cell(&self, rgba: &[u8]) -> Result<Vec<u8>, String> {
        let q = self.quality.clamp(1, 100) as f32;

        let mut config = WebPConfig::new().map_err(|_| "Failed to initialize WebPConfig".to_string())?;
        config.quality = q;
        config.method = self.method.clamp(4, 6);

        let memory = if self.grayscale {
            // Convert RGBA to 8-bit Grayscale replicated into RGB/RGBA
            let mut gray_rgba = Vec::with_capacity(self.cell_width * self.cell_height * 4);
            for chunk in rgba.chunks_exact(4) {
                let r = chunk[0] as f32;
                let g = chunk[1] as f32;
                let b = chunk[2] as f32;
                let y = (0.299 * r + 0.587 * g + 0.114 * b) as u8;
                gray_rgba.push(y);
                gray_rgba.push(y);
                gray_rgba.push(y);
                gray_rgba.push(255);
            }
            let encoder = Encoder::from_rgba(&gray_rgba, self.cell_width as u32, self.cell_height as u32);
            encoder.encode_advanced(&config).map_err(|e| format!("WebP encode error: {:?}", e))?
        } else {
            let encoder = Encoder::from_rgba(rgba, self.cell_width as u32, self.cell_height as u32);
            encoder.encode_advanced(&config).map_err(|e| format!("WebP encode error: {:?}", e))?
        };

        Ok(memory.to_vec())
    }
}
