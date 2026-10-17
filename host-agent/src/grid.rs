pub const SCREEN_WIDTH: usize = 1920;
pub const SCREEN_HEIGHT: usize = 1080;
pub const GRID_COLS: usize = 48;
pub const GRID_ROWS: usize = 27;
pub const CELL_WIDTH: usize = 40;
pub const CELL_HEIGHT: usize = 40;
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

#[derive(Clone, Debug)]
pub struct DirtyCell {
    pub row: usize,
    pub col: usize,
    pub cell_id: String,
    pub version: u32,
    pub rgba_data: Vec<u8>, // 40x40 RGBA bytes = 6,400 bytes
}

pub struct GridDiffTracker {
    cell_hashes: [u64; TOTAL_CELLS],
    cell_versions: [u32; TOTAL_CELLS],
    first_frame: bool,
}

impl GridDiffTracker {
    pub fn new() -> Self {
        Self {
            cell_hashes: [0; TOTAL_CELLS],
            cell_versions: [0; TOTAL_CELLS],
            first_frame: true,
        }
    }

    /// Fast 64-bit FNV-1a hash of a 40x40 pixel RGBA chunk
    fn hash_cell(frame: &[u8], start_x: usize, start_y: usize, stride: usize) -> u64 {
        const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
        const FNV_PRIME: u64 = 0x100000001b3;

        let mut hash = FNV_OFFSET_BASIS;

        for y in 0..CELL_HEIGHT {
            let row_offset = (start_y + y) * stride + (start_x * 4);
            let row_slice = &frame[row_offset..row_offset + (CELL_WIDTH * 4)];
            for &byte in row_slice {
                hash ^= byte as u64;
                hash = hash.wrapping_mul(FNV_PRIME);
            }
        }
        hash
    }

    /// Extracts 40x40 RGBA pixels for a single cell into a contiguous buffer
    pub fn extract_cell_rgba(frame: &[u8], start_x: usize, start_y: usize, stride: usize) -> Vec<u8> {
        let mut cell_data = Vec::with_capacity(CELL_WIDTH * CELL_HEIGHT * 4);
        for y in 0..CELL_HEIGHT {
            let row_offset = (start_y + y) * stride + (start_x * 4);
            let row_slice = &frame[row_offset..row_offset + (CELL_WIDTH * 4)];
            cell_data.extend_from_slice(row_slice);
        }
        cell_data
    }

    /// Diffs the current 1920x1080 RGBA frame against the previous frame.
    /// Returns only dirty cells (or all 1296 if force_full_sync is true).
    pub fn diff_frame(&mut self, frame: &[u8], force_full_sync: bool) -> (Vec<DirtyCell>, bool) {
        let is_full = force_full_sync || self.first_frame;
        self.first_frame = false;

        let stride = SCREEN_WIDTH * 4;
        let mut dirty_cells = Vec::new();

        for r in 0..GRID_ROWS {
            let start_y = r * CELL_HEIGHT;
            for c in 0..GRID_COLS {
                let start_x = c * CELL_WIDTH;
                let cell_idx = r * GRID_COLS + c;

                let h = Self::hash_cell(frame, start_x, start_y, stride);
                if is_full || h != self.cell_hashes[cell_idx] {
                    self.cell_hashes[cell_idx] = h;
                    self.cell_versions[cell_idx] = self.cell_versions[cell_idx].wrapping_add(1);

                    let rgba = Self::extract_cell_rgba(frame, start_x, start_y, stride);
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

        (dirty_cells, is_full)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_dimensions_and_naming() {
        assert_eq!(GRID_COLS * GRID_ROWS, TOTAL_CELLS);
        assert_eq!(TOTAL_CELLS, 1296);

        // Check first and last cell IDs
        assert_eq!(get_cell_id(0, 0), "001");
        assert_eq!(get_cell_id(3, 16), "C17");
        assert_eq!(get_cell_id(26, 47), "Z48");
    }

    #[test]
    fn test_grid_diffing_dirty_detection() {
        let mut tracker = GridDiffTracker::new();
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
        let c17_y = 3 * CELL_HEIGHT + 5;
        let c17_x = 16 * CELL_WIDTH + 5;
        let px_idx = c17_y * stride + c17_x * 4;
        frame3[px_idx] = 255; // Modify byte

        let (dirty3, is_full3) = tracker.diff_frame(&frame3, false);
        assert!(!is_full3);
        assert_eq!(dirty3.len(), 1);
        assert_eq!(dirty3[0].cell_id, "C17");
        assert_eq!(dirty3[0].row, 3);
        assert_eq!(dirty3[0].col, 16);
    }
}
