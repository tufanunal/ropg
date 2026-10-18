pub const SCREEN_WIDTH: usize = 1920;
pub const SCREEN_HEIGHT: usize = 1080;
#[allow(dead_code)]
pub const GRID_COLS: usize = 48;
#[allow(dead_code)]
pub const GRID_ROWS: usize = 27;
#[allow(dead_code)]
pub const CELL_WIDTH: usize = 40;
#[allow(dead_code)]
pub const CELL_HEIGHT: usize = 40;
#[allow(dead_code)]
pub const TOTAL_CELLS: usize = 1296;

pub const ROW_LABELS: [char; 27] = [
    '0', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I',
    'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z'
];

pub fn get_cell_id(row: usize, col: usize) -> String {
    let r = if row < ROW_LABELS.len() { ROW_LABELS[row] } else { '?' };
    format!("{}{:02}", r, col + 1)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridConfig {
    pub name: &'static str,
    pub cell_width: usize,
    pub cell_height: usize,
    pub cell_size: usize,
    pub cols: usize,
    pub rows: usize,
    pub total_cells: usize,
}

impl GridConfig {
    pub fn from_preset(name: &str) -> Self {
        match name {
            "60" | "60x60" => Self { name: "60x60", cell_width: 60, cell_height: 60, cell_size: 60, cols: 32, rows: 18, total_cells: 576 },
            "120" | "120x120" => Self { name: "120x120", cell_width: 120, cell_height: 120, cell_size: 120, cols: 16, rows: 9, total_cells: 144 },
            "160" | "160x180" => Self { name: "160x180", cell_width: 160, cell_height: 180, cell_size: 160, cols: 12, rows: 6, total_cells: 72 },
            "240" | "240x270" => Self { name: "240x270", cell_width: 240, cell_height: 270, cell_size: 240, cols: 8, rows: 4, total_cells: 32 },
            "384" | "284" | "384x360" | "284x360" => Self { name: "384x360", cell_width: 384, cell_height: 360, cell_size: 384, cols: 5, rows: 3, total_cells: 15 },
            "640" | "640x540" => Self { name: "640x540", cell_width: 640, cell_height: 540, cell_size: 640, cols: 3, rows: 2, total_cells: 6 },
            _ => Self { name: "40x40", cell_width: 40, cell_height: 40, cell_size: 40, cols: 48, rows: 27, total_cells: 1296 },
        }
    }

    pub fn from_cell_size(size: usize) -> Self {
        match size {
            60 => Self::from_preset("60x60"),
            120 => Self::from_preset("120x120"),
            160 => Self::from_preset("160x180"),
            240 => Self::from_preset("240x270"),
            384 | 284 => Self::from_preset("384x360"),
            640 => Self::from_preset("640x540"),
            _ => Self::from_preset("40x40"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct DirtyCell {
    pub row: usize,
    pub col: usize,
    pub cell_id: String,
    pub version: u32,
    pub rgba_data: Vec<u8>,
}

pub struct GridDiffTracker {
    pub config: GridConfig,
    cell_hashes: Vec<u64>,
    cell_versions: Vec<u32>,
    first_frame: bool,
    prev_frame: Vec<u8>,
}

impl GridDiffTracker {
    #[allow(dead_code)]
    pub fn new(cell_size: usize) -> Self {
        let config = GridConfig::from_cell_size(cell_size);
        Self {
            cell_hashes: vec![0; config.total_cells],
            cell_versions: vec![0; config.total_cells],
            config,
            first_frame: true,
            prev_frame: Vec::new(),
        }
    }

    pub fn new_with_preset(preset: &str) -> Self {
        let config = GridConfig::from_preset(preset);
        Self {
            cell_hashes: vec![0; config.total_cells],
            cell_versions: vec![0; config.total_cells],
            config,
            first_frame: true,
            prev_frame: Vec::new(),
        }
    }

    pub fn set_preset(&mut self, preset: &str) {
        let new_config = GridConfig::from_preset(preset);
        if self.config != new_config {
            self.config = new_config;
            self.cell_hashes = vec![0; new_config.total_cells];
            self.cell_versions = vec![0; new_config.total_cells];
            self.first_frame = true;
            self.prev_frame.clear();
        }
    }

    #[allow(dead_code)]
    pub fn set_cell_size(&mut self, cell_size: usize) {
        let new_config = GridConfig::from_cell_size(cell_size);
        if self.config != new_config {
            self.config = new_config;
            self.cell_hashes = vec![0; new_config.total_cells];
            self.cell_versions = vec![0; new_config.total_cells];
            self.first_frame = true;
            self.prev_frame.clear();
        }
    }

    /// Fast 64-bit word chunk hash of a pixel RGBA chunk
    pub fn hash_cell(frame: &[u8], start_x: usize, start_y: usize, stride: usize, cell_w: usize, cell_h: usize) -> u64 {
        const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
        const FNV_PRIME: u64 = 0x100000001b3;

        let mut hash = FNV_OFFSET_BASIS;

        for y in 0..cell_h {
            let row_offset = (start_y + y) * stride + (start_x * 4);
            let row_slice = &frame[row_offset..row_offset + (cell_w * 4)];

            // Process in 8-byte chunks (SIMD/word level)
            let mut chunks = row_slice.chunks_exact(8);
            for chunk in chunks.by_ref() {
                let val = u64::from_ne_bytes(chunk.try_into().unwrap());
                hash ^= val;
                hash = hash.wrapping_mul(FNV_PRIME);
            }
            // Remainder bytes
            for &byte in chunks.remainder() {
                hash ^= byte as u64;
                hash = hash.wrapping_mul(FNV_PRIME);
            }
        }
        hash
    }

    /// Fast early-exit comparison of a cell against the previous frame.
    /// Returns true if all pixels match exactly (memcmp SIMD equality).
    fn is_cell_unchanged(&self, frame: &[u8], start_x: usize, start_y: usize, stride: usize, cell_w: usize, cell_h: usize) -> bool {
        if self.prev_frame.len() != frame.len() {
            return false;
        }

        let bytes_per_row = cell_w * 4;
        for y in 0..cell_h {
            let row_offset = (start_y + y) * stride + (start_x * 4);
            let curr_row = &frame[row_offset..row_offset + bytes_per_row];
            let prev_row = &self.prev_frame[row_offset..row_offset + bytes_per_row];
            if curr_row != prev_row {
                return false;
            }
        }
        true
    }

    /// Extracts RGBA pixels for a single cell into a contiguous buffer
    pub fn extract_cell_rgba(frame: &[u8], start_x: usize, start_y: usize, stride: usize, cell_w: usize, cell_h: usize) -> Vec<u8> {
        let mut cell_data = Vec::with_capacity(cell_w * cell_h * 4);
        for y in 0..cell_h {
            let row_offset = (start_y + y) * stride + (start_x * 4);
            let row_slice = &frame[row_offset..row_offset + (cell_w * 4)];
            cell_data.extend_from_slice(row_slice);
        }
        cell_data
    }

    /// Diffs the current 1920x1080 RGBA frame against the previous frame.
    /// Returns only dirty cells (or all cells if force_full_sync is true).
    pub fn diff_frame(&mut self, frame: &[u8], force_full_sync: bool) -> (Vec<DirtyCell>, bool) {
        let is_full = force_full_sync || self.first_frame || self.prev_frame.len() != frame.len();
        self.first_frame = false;

        let stride = SCREEN_WIDTH * 4;
        let mut dirty_cells = Vec::new();
        let cell_w = self.config.cell_width;
        let cell_h = self.config.cell_height;
        let rows = self.config.rows;
        let cols = self.config.cols;

        for r in 0..rows {
            let start_y = r * cell_h;
            for c in 0..cols {
                let start_x = c * cell_w;
                let cell_idx = r * cols + c;

                // Early exit: if not full sync, check SIMD equality against previous frame
                if !is_full && self.is_cell_unchanged(frame, start_x, start_y, stride, cell_w, cell_h) {
                    continue;
                }

                let h = Self::hash_cell(frame, start_x, start_y, stride, cell_w, cell_h);
                if is_full || h != self.cell_hashes[cell_idx] {
                    self.cell_hashes[cell_idx] = h;
                    self.cell_versions[cell_idx] = self.cell_versions[cell_idx].wrapping_add(1);

                    let rgba = Self::extract_cell_rgba(frame, start_x, start_y, stride, cell_w, cell_h);
                    dirty_cells.push(DirtyCell {
                        row: r,
                        col: c,
                        cell_id: get_cell_id(r, c),
                        version: self.cell_versions[cell_idx],
                        rgba_data: rgba,
                    });
                }
            }
        }

        // Cache previous frame for fast SIMD comparison in the next iteration
        if self.prev_frame.len() == frame.len() {
            self.prev_frame.copy_from_slice(frame);
        } else {
            self.prev_frame = frame.to_vec();
        }

        (dirty_cells, is_full)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_configurations() {
        let cfg40 = GridConfig::from_preset("40x40");
        assert_eq!(cfg40.cols, 48);
        assert_eq!(cfg40.rows, 27);
        assert_eq!(cfg40.total_cells, 1296);

        let cfg60 = GridConfig::from_preset("60x60");
        assert_eq!(cfg60.cols, 32);
        assert_eq!(cfg60.rows, 18);
        assert_eq!(cfg60.total_cells, 576);

        let cfg120 = GridConfig::from_preset("120x120");
        assert_eq!(cfg120.cols, 16);
        assert_eq!(cfg120.rows, 9);
        assert_eq!(cfg120.total_cells, 144);

        let cfg160 = GridConfig::from_preset("160x180");
        assert_eq!(cfg160.cols, 12);
        assert_eq!(cfg160.rows, 6);
        assert_eq!(cfg160.total_cells, 72);

        let cfg240 = GridConfig::from_preset("240x270");
        assert_eq!(cfg240.cols, 8);
        assert_eq!(cfg240.rows, 4);
        assert_eq!(cfg240.total_cells, 32);

        let cfg384 = GridConfig::from_preset("384x360");
        assert_eq!(cfg384.cols, 5);
        assert_eq!(cfg384.rows, 3);
        assert_eq!(cfg384.total_cells, 15);

        // Also test alias for 284x360
        let cfg284 = GridConfig::from_preset("284x360");
        assert_eq!(cfg284.cols, 5);
        assert_eq!(cfg284.rows, 3);
        assert_eq!(cfg284.total_cells, 15);

        let cfg640 = GridConfig::from_preset("640x540");
        assert_eq!(cfg640.cols, 3);
        assert_eq!(cfg640.rows, 2);
        assert_eq!(cfg640.total_cells, 6);
    }

    #[test]
    fn test_grid_diffing_dirty_detection() {
        let mut tracker = GridDiffTracker::new(40);
        let stride = SCREEN_WIDTH * 4;

        // Frame 1: blank frame -> should trigger full sync (1296 cells)
        let frame1 = vec![0u8; SCREEN_WIDTH * SCREEN_HEIGHT * 4];
        let (dirty1, is_full1) = tracker.diff_frame(&frame1, false);
        assert!(is_full1);
        assert_eq!(dirty1.len(), 1296);

        // Frame 2: identical frame -> 0 dirty cells
        let (dirty2, is_full2) = tracker.diff_frame(&frame1, false);
        assert!(!is_full2);
        assert_eq!(dirty2.len(), 0);

        // Frame 3: modify a single pixel in cell C17 (row 3, col 16)
        let mut frame3 = frame1.clone();
        let c17_y = 3 * 40 + 5;
        let c17_x = 16 * 40 + 5;
        let px_idx = c17_y * stride + c17_x * 4;
        frame3[px_idx] = 255; // Modify byte

        let (dirty3, is_full3) = tracker.diff_frame(&frame3, false);
        assert!(!is_full3);
        assert_eq!(dirty3.len(), 1);
        assert_eq!(dirty3[0].cell_id, "C17");
        assert_eq!(dirty3[0].row, 3);
        assert_eq!(dirty3[0].col, 16);
    }

    #[test]
    fn test_non_square_grid_diffing() {
        let mut tracker = GridDiffTracker::new_with_preset("160x180");
        let stride = SCREEN_WIDTH * 4;

        let frame1 = vec![0u8; SCREEN_WIDTH * SCREEN_HEIGHT * 4];
        let (dirty1, is_full1) = tracker.diff_frame(&frame1, false);
        assert!(is_full1);
        assert_eq!(dirty1.len(), 72); // 12x6 = 72

        // Modify pixel in row 1, col 2 -> y = 180 + 10, x = 320 + 10
        let mut frame2 = frame1.clone();
        let px_idx = (180 + 10) * stride + (320 + 10) * 4;
        frame2[px_idx] = 200;

        let (dirty2, is_full2) = tracker.diff_frame(&frame2, false);
        assert!(!is_full2);
        assert_eq!(dirty2.len(), 1);
        assert_eq!(dirty2[0].row, 1);
        assert_eq!(dirty2[0].col, 2);
    }
}
