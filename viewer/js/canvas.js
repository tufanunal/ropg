/**
 * Rogp Canvas Blit Engine
 * Manages the 1920x1080 HTML5 Canvas and blits dynamic cell updates (40x40, 60x60, 120x120).
 */

import { SCREEN_WIDTH, SCREEN_HEIGHT, ROW_LABELS } from './protocol.js';

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

    this.gridPreset = '40x40';
    this.cellSize = 40;
    this.cellWidth = 40;
    this.cellHeight = 40;
    this.gridCols = 48;
    this.gridRows = 27;

    // Cell version tracking: Map<cellId, versionNumber>
    this.cellVersions = new Map();
    this.totalCellsUpdated = 0;
    this.showGrid = false;
    this.flashUpdatedCells = false;

    // Fill initial canvas with clean dark backdrop
    this.clear();
  }

  setGridPreset(preset) {
    const str = String(preset || '40x40').toLowerCase();
    let w = 40, h = 40, cols = 48, rows = 27;

    if (str === '60x60' || str === '60') {
      w = 60; h = 60; cols = 32; rows = 18;
    } else if (str === '120x120' || str === '120') {
      w = 120; h = 120; cols = 16; rows = 9;
    } else if (str === '160x180' || str === '160') {
      w = 160; h = 180; cols = 12; rows = 6;
    } else if (str === '240x270' || str === '240') {
      w = 240; h = 270; cols = 8; rows = 4;
    } else if (str === '384x360' || str === '284x360' || str === '384' || str === '284') {
      w = 384; h = 360; cols = 5; rows = 3;
    } else if (str === '640x540' || str === '640') {
      w = 640; h = 540; cols = 3; rows = 2;
    } else {
      w = 40; h = 40; cols = 48; rows = 27;
    }

    const presetName = `${w}x${h}`;
    if (this.gridPreset === presetName) return;

    this.gridPreset = presetName;
    this.cellSize = w;
    this.cellWidth = w;
    this.cellHeight = h;
    this.gridCols = cols;
    this.gridRows = rows;
    this.cellVersions.clear();
    if (this.showGrid) {
      this.renderGridOverlay();
    }
  }

  setGridSize(size) {
    this.setGridPreset(size);
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
    const x = cell.col * this.cellWidth;
    const y = cell.row * this.cellHeight;

    try {
      // Use createImageBitmap for fast off-thread decode
      const bitmap = await createImageBitmap(cell.blob);
      this.ctx.drawImage(bitmap, x, y, this.cellWidth, this.cellHeight);
      bitmap.close();

      if (this.flashUpdatedCells && this.overlayCtx) {
        this.overlayCtx.fillStyle = 'rgba(56, 189, 248, 0.4)'; // Sky blue highlight
        this.overlayCtx.fillRect(x, y, this.cellWidth, this.cellHeight);
        setTimeout(() => {
          this.overlayCtx.clearRect(x, y, this.cellWidth, this.cellHeight);
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
    this.overlayCtx.clearRect(0, 0, SCREEN_WIDTH, SCREEN_HEIGHT);
    this.overlayCtx.save();
    this.overlayCtx.strokeStyle = 'rgba(148, 163, 184, 0.2)'; // Thin grid lines
    this.overlayCtx.lineWidth = 1;

    // Draw vertical lines
    for (let c = 0; c <= this.gridCols; c++) {
      const x = c * this.cellWidth;
      this.overlayCtx.beginPath();
      this.overlayCtx.moveTo(x, 0);
      this.overlayCtx.lineTo(x, SCREEN_HEIGHT);
      this.overlayCtx.stroke();
    }

    // Draw horizontal lines
    for (let r = 0; r <= this.gridRows; r++) {
      const y = r * this.cellHeight;
      this.overlayCtx.beginPath();
      this.overlayCtx.moveTo(0, y);
      this.overlayCtx.lineTo(SCREEN_WIDTH, y);
      this.overlayCtx.stroke();
    }

    // Draw subtle row/column headers
    this.overlayCtx.fillStyle = 'rgba(255, 255, 255, 0.35)';
    this.overlayCtx.font = '10px monospace';
    const step = this.gridCols <= 16 ? 1 : (this.gridCols <= 32 ? 2 : 4);
    for (let r = 0; r < this.gridRows; r++) {
      for (let c = 0; c < this.gridCols; c += step) {
        const id = `${ROW_LABELS[r] || '?'}${String(c + 1).padStart(2, '0')}`;
        this.overlayCtx.fillText(id, c * this.cellWidth + 4, r * this.cellHeight + 14);
      }
    }

    this.overlayCtx.restore();
  }
}
