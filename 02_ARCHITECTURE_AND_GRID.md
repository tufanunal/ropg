# Screen Architecture & Grid System

## Grid Slicing Logic
The system assumes a fixed 1920x1080 (FHD) resolution for all hosts.
The screen MUST be divided into a perfect grid:
- **Horizontal (X):** 48 columns. (1920 / 48 = 40 pixels width per cell)
- **Vertical (Y):** 27 rows. (1080 / 27 = 40 pixels height per cell)
- **Total Cells:** 1296 cells per frame.

### Cell Naming Convention
- **Rows (27):** Defined by character `0` + English Alphabet (`A` to `Z`). (e.g., 0, A, B, C...)
- **Columns (48):** Defined by two-digit numbers `01` to `48`.
- **Example Cell ID:** `C17` (Row C, Column 17).

## Diffing & Image Processing
1. The Host Agent (Y) captures the 1920x1080 screen at a base interval (default 1 second).
2. It slices the screen into 1296 cells.
3. It compares each cell against the previous state.
4. **ONLY** changed cells are converted to WebP format.
5. Viewer (C) retains the last known state of unmodified cells on the screen.

### Bandwidth Saving Configuration
The Host Agent (Y) MUST support configurable WebP settings:
- `quality`: e.g., 80% (adjustable).
- `color_mode`: Color or Grayscale.

## Payload Modes
The system MUST support two selectable modes for transmitting cell data from Host to Relay:
1. **Mode A (JSON + Base64):** Cell coordinates and Base64-encoded WebP strings are packed into a JSON array. Easy to debug, but ~33% overhead.
2. **Mode B (Binary Packet):** Cell coordinates, data length headers, and raw WebP bytes are packed into a single binary payload to eliminate Base64 overhead.
*(Important: Multiple cells MUST be batched into a single POST request to minimize HTTP overhead).*
