/**
 * Rogp Canvas Blit Engine
 * Manages the 1920x1080 HTML5 Canvas and blits 40x40 cell updates.
 */

import { SCREEN_WIDTH, SCREEN_HEIGHT, CELL_WIDTH, CELL_HEIGHT, GRID_COLS, GRID_ROWS, ROW_LABELS } from './protocol.js';

export class ScreenCanvas {
  constructor(canvasElement, overlayCanvasElement) {
    this.canvas = canvasElement;
    this.ctx = this.canvas.getContext('2d', { alpha: false });
    this.overlayCanvas = overlayCanvasElement;
    this.overlayCtx = this.overlayCanvas ? this.overlayCanvas.getContext('2d') : null;

    this.canvas.width = SCREEN_WIDTH;
    this.canvas.height = SCREEN_HEIGHT;
    if (this.overlayCanvas) {
      this.overlayCanvas.width = SCREEN_WIDTH;
      this.overlayCanvas.height = SCREEN_HEIGHT;
    }

    // Cell version tracking: Map<cellId, versionNumber>
    this.cellVersions = new Map();
    this.totalCellsUpdated = 0;
    this.showGrid = false;
    this.flashUpdatedCells = false;

    // Fill initial canvas with clean dark backdrop
    this.clear();
  }

  clear() {
    this.ctx.fillStyle = '#0f172a'; // Slate 900
    this.ctx.fillRect(0, 0, SCREEN_WIDTH, SCREEN_HEIGHT);
    this.cellVersions.clear();
    this.drawEmptyState();
  }

  drawEmptyState() {
    this.ctx.save();
    this.ctx.fillStyle = '#475569';
    this.ctx.font = '24px -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif';
    this.ctx.textAlign = 'center';
    this.ctx.fillText('Waiting for Host Screen Data...', SCREEN_WIDTH / 2, SCREEN_HEIGHT / 2);
    this.ctx.restore();
  }

  /**
   * Apply an array of updated cells { cellId, row, col, version, blob }
   */
  async applyCells(cells, isFullSync = false) {
    if (isFullSync) {
      this.cellVersions.clear();
    }

    const updates = [];
    for (const cell of cells) {
      this.cellVersions.set(cell.cellId, cell.version);
      updates.push(this.renderCell(cell));
    }

    await Promise.all(updates);
    this.totalCellsUpdated += cells.length;

    if (this.showGrid) {
      this.renderGridOverlay();
    }
  }

  async renderCell(cell) {
    const x = cell.col * CELL_WIDTH;
    const y = cell.row * CELL_HEIGHT;

    try {
      // Use createImageBitmap for fast off-thread decode
      const bitmap = await createImageBitmap(cell.blob);
      this.ctx.drawImage(bitmap, x, y, CELL_WIDTH, CELL_HEIGHT);
      bitmap.close();

      if (this.flashUpdatedCells && this.overlayCtx) {
        this.overlayCtx.fillStyle = 'rgba(56, 189, 248, 0.4)'; // Sky blue highlight
        this.overlayCtx.fillRect(x, y, CELL_WIDTH, CELL_HEIGHT);
        setTimeout(() => {
          this.overlayCtx.clearRect(x, y, CELL_WIDTH, CELL_HEIGHT);
        }, 120);
      }
    } catch (err) {
      console.warn(`Failed to render cell ${cell.cellId}:`, err);
    }
  }

  toggleGrid(enabled) {
    this.showGrid = enabled;
    if (!this.overlayCtx) return;
    this.overlayCtx.clearRect(0, 0, SCREEN_WIDTH, SCREEN_HEIGHT);
    if (this.showGrid) {
      this.renderGridOverlay();
    }
  }

  renderGridOverlay() {
    if (!this.overlayCtx) return;
    this.overlayCtx.save();
    this.overlayCtx.strokeStyle = 'rgba(148, 163, 184, 0.2)'; // Thin grid lines
    this.overlayCtx.lineWidth = 1;

    // Draw vertical lines
    for (let c = 0; c <= GRID_COLS; c++) {
      const x = c * CELL_WIDTH;
      this.overlayCtx.beginPath();
      this.overlayCtx.moveTo(x, 0);
      this.overlayCtx.lineTo(x, SCREEN_HEIGHT);
      this.overlayCtx.stroke();
    }

    // Draw horizontal lines
    for (let r = 0; r <= GRID_ROWS; r++) {
      const y = r * CELL_HEIGHT;
      this.overlayCtx.beginPath();
      this.overlayCtx.moveTo(0, y);
      this.overlayCtx.lineTo(SCREEN_WIDTH, y);
      this.overlayCtx.stroke();
    }

    // Draw subtle row/column headers
    this.overlayCtx.fillStyle = 'rgba(255, 255, 255, 0.35)';
    this.overlayCtx.font = '10px monospace';
    for (let r = 0; r < GRID_ROWS; r++) {
      for (let c = 0; c < GRID_COLS; c += 4) {
        const id = `${ROW_LABELS[r]}${String(c + 1).padStart(2, '0')}`;
        this.overlayCtx.fillText(id, c * CELL_WIDTH + 4, r * CELL_HEIGHT + 14);
      }
    }

    this.overlayCtx.restore();
  }
}
