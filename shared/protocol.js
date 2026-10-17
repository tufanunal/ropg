/**
 * Rogp Shared Protocol Definitions and Constants
 * 
 * Screen: Fixed 1920x1080 FHD
 * Grid: 48 columns x 27 rows (1296 cells)
 * Cell Size: 40x40 pixels
 */

const SCREEN_WIDTH = 1920;
const SCREEN_HEIGHT = 1080;
const GRID_COLS = 48;
const GRID_ROWS = 27;
const CELL_WIDTH = 40;
const CELL_HEIGHT = 40;
const TOTAL_CELLS = 1296;

// Row labels: 0, A, B, C, ... Z (27 rows total)
const ROW_LABELS = ['0', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z'];

// Binary Magic: "ROGP" (0x52, 0x4F, 0x47, 0x50)
const BINARY_MAGIC = 0x524F4750;
const BINARY_VERSION = 1;

/**
 * Returns cell ID from row (0-26) and col (0-47)
 * e.g., row=0, col=0 -> "001"
 * e.g., row=3, col=16 -> "C17"
 */
function getCellId(row, col) {
  if (row < 0 || row >= GRID_ROWS || col < 0 || col >= GRID_COLS) {
    throw new Error(`Invalid grid coords: row=${row}, col=${col}`);
  }
  const rowChar = ROW_LABELS[row];
  const colStr = String(col + 1).padStart(2, '0');
  return `${rowChar}${colStr}`;
}

/**
 * Parses cell ID into row (0-26) and col (0-47)
 * e.g., "001" -> { row: 0, col: 0 }
 * e.g., "C17" -> { row: 3, col: 16 }
 */
function parseCellId(cellId) {
  if (!cellId || cellId.length < 3) {
    throw new Error(`Invalid cellId: ${cellId}`);
  }
  const rowChar = cellId.charAt(0);
  const colStr = cellId.substring(1);
  const row = ROW_LABELS.indexOf(rowChar);
  const col = parseInt(colStr, 10) - 1;
  if (row === -1 || isNaN(col) || col < 0 || col >= GRID_COLS) {
    throw new Error(`Invalid cellId format: ${cellId}`);
  }
  return { row, col };
}

/**
 * Calculates pixel rectangle for a cell
 */
function getCellRect(row, col) {
  return {
    x: col * CELL_WIDTH,
    y: row * CELL_HEIGHT,
    width: CELL_WIDTH,
    height: CELL_HEIGHT
  };
}

module.exports = {
  SCREEN_WIDTH,
  SCREEN_HEIGHT,
  GRID_COLS,
  GRID_ROWS,
  CELL_WIDTH,
  CELL_HEIGHT,
  TOTAL_CELLS,
  ROW_LABELS,
  BINARY_MAGIC,
  BINARY_VERSION,
  getCellId,
  parseCellId,
  getCellRect
};
