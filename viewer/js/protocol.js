/**
 * Rogp Viewer Protocol Decoder
 * Handles decoding of Mode A (JSON + Base64) and Mode B (Binary ROGP)
 */

export const SCREEN_WIDTH = 1920;
export const SCREEN_HEIGHT = 1080;
export const GRID_COLS = 48;
export const GRID_ROWS = 27;
export const CELL_WIDTH = 40;
export const CELL_HEIGHT = 40;
export const TOTAL_CELLS = 1296;

export const ROW_LABELS = ['0', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z'];

export function getCellId(row, col) {
  const rowChar = ROW_LABELS[row] || '?';
  const colStr = String(col + 1).padStart(2, '0');
  return `${rowChar}${colStr}`;
}

export function parseCellId(cellId) {
  const rowChar = cellId.charAt(0);
  const colStr = cellId.substring(1);
  const row = ROW_LABELS.indexOf(rowChar);
  const col = parseInt(colStr, 10) - 1;
  return { row, col };
}

/**
 * Decode Mode B: Binary Packet
 * Binary Layout:
 * [0..3]   Magic: 0x52, 0x4F, 0x47, 0x50 ("ROGP")
 * [4]      Version: uint8 (1)
 * [5]      Flags: uint8 (bit 0: isFullSync, bit 1: isGrayscale)
 * [6..7]   Cell Count: uint16 big endian
 * [8..15]  Timestamp: uint64 big endian (ms)
 * 
 * Each Cell:
 * [0]      Row: uint8 (0..26)
 * [1]      Col: uint8 (0..47)
 * [2..5]   Version: uint32 big endian
 * [6..9]   Data Length: uint32 big endian (N)
 * [10..10+N-1] WebP Raw Bytes
 */
export async function decodeBinaryPacket(arrayBuffer) {
  const view = new DataView(arrayBuffer);
  if (arrayBuffer.byteLength < 16) {
    throw new Error('Binary packet too short');
  }

  // Check Magic "ROGP"
  const magic = view.getUint32(0, false);
  if (magic !== 0x524F4750) {
    throw new Error('Invalid magic header');
  }

  const version = view.getUint8(4);
  const flags = view.getUint8(5);
  const isFullSync = (flags & 0x01) !== 0;
  const isGrayscale = (flags & 0x02) !== 0;
  const cellCount = view.getUint16(6, false);
  const timestamp = Number(view.getBigUint64(8, false));

  const cells = [];
  let offset = 16;

  for (let i = 0; i < cellCount; i++) {
    if (offset + 10 > arrayBuffer.byteLength) break;

    const row = view.getUint8(offset);
    const col = view.getUint8(offset + 1);
    const cellVersion = view.getUint32(offset + 2, false);
    const dataLen = view.getUint32(offset + 6, false);
    offset += 10;

    if (offset + dataLen > arrayBuffer.byteLength) {
      console.warn(`Truncated cell data for cell ${row},${col}`);
      break;
    }

    const webpBytes = new Uint8Array(arrayBuffer, offset, dataLen);
    offset += dataLen;

    const blob = new Blob([webpBytes], { type: 'image/webp' });
    const cellId = getCellId(row, col);

    cells.push({
      cellId,
      row,
      col,
      version: cellVersion,
      blob
    });
  }

  return {
    isFullSync,
    isGrayscale,
    timestamp,
    cellCount: cells.length,
    cells
  };
}

/**
 * Decode Mode A: JSON Packet
 */
export async function decodeJsonPacket(jsonData) {
  const isFullSync = !!jsonData.full_sync;
  const timestamp = jsonData.timestamp || Date.now();
  const rawCells = jsonData.cells || [];
  const cells = [];

  for (const item of rawCells) {
    const { row, col } = parseCellId(item.id);
    let blob;
    if (item.data.startsWith('data:')) {
      const res = await fetch(item.data);
      blob = await res.blob();
    } else {
      // Raw base64 string
      const byteCharacters = atob(item.data);
      const byteNumbers = new Array(byteCharacters.length);
      for (let i = 0; i < byteCharacters.length; i++) {
        byteNumbers[i] = byteCharacters.charCodeAt(i);
      }
      blob = new Blob([new Uint8Array(byteNumbers)], { type: 'image/webp' });
    }

    cells.push({
      cellId: item.id,
      row,
      col,
      version: item.v || 0,
      blob
    });
  }

  return {
    isFullSync,
    timestamp,
    cellCount: cells.length,
    cells
  };
}
