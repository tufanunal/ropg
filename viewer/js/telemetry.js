/**
 * Rogp Telemetry Dashboard & Performance Monitor
 * Tracks real-time network throughput, cell update rates, HTTP requests/sec, and latencies.
 */

export class TelemetryMonitor {
  constructor(containerElement) {
    this.container = containerElement;

    // Metrics rolling window
    this.bytesWindow = [];
    this.cellsWindow = [];
    this.requestsWindow = [];
    this.latenciesWindow = [];

    this.currentMode = 'binary'; // 'json' or 'binary'
    this.totalCellsUpdated = 0;
    this.totalBytesReceived = 0;
    this.totalRequestsMade = 0;

    // DOM Elements cache
    this.dom = {
      bandwidth: document.getElementById('stat-bandwidth'),
      cellsSec: document.getElementById('stat-cells-sec'),
      reqSec: document.getElementById('stat-req-sec'),
      latency: document.getElementById('stat-latency'),
      modeBadge: document.getElementById('stat-mode-badge'),
      totalBytes: document.getElementById('stat-total-bytes'),
      totalCells: document.getElementById('stat-total-cells')
    };

    // Update DOM every 500ms
    setInterval(() => this.tick(), 500);
  }

  recordRequest(byteLength, cellCount, latencyMs, mode) {
    const now = performance.now();
    this.bytesWindow.push({ time: now, bytes: byteLength });
    this.cellsWindow.push({ time: now, count: cellCount });
    this.requestsWindow.push({ time: now });
    if (typeof latencyMs === 'number' && latencyMs >= 0) {
      this.latenciesWindow.push({ time: now, latency: latencyMs });
    }

    if (mode) {
      this.currentMode = mode;
    }

    this.totalBytesReceived += byteLength;
    this.totalCellsUpdated += cellCount;
    this.totalRequestsMade++;
  }

  tick() {
    const now = performance.now();
    const windowMs = 1500; // 1.5s sliding window for smooth rates

    // Clean old entries
    this.bytesWindow = this.bytesWindow.filter(e => now - e.time <= windowMs);
    this.cellsWindow = this.cellsWindow.filter(e => now - e.time <= windowMs);
    this.requestsWindow = this.requestsWindow.filter(e => now - e.time <= windowMs);
    this.latenciesWindow = this.latenciesWindow.filter(e => now - e.time <= windowMs);

    // Sum and calculate per-second rates
    const sumBytes = this.bytesWindow.reduce((acc, curr) => acc + curr.bytes, 0);
    const sumCells = this.cellsWindow.reduce((acc, curr) => acc + curr.count, 0);
    const countReqs = this.requestsWindow.length;

    const seconds = windowMs / 1000;
    const kbPerSec = (sumBytes / 1024 / seconds).toFixed(1);
    const cellsPerSec = Math.round(sumCells / seconds);
    const reqPerSec = (countReqs / seconds).toFixed(1);

    let avgLatency = 0;
    if (this.latenciesWindow.length > 0) {
      const sumLatency = this.latenciesWindow.reduce((acc, c) => acc + c.latency, 0);
      avgLatency = Math.round(sumLatency / this.latenciesWindow.length);
    }

    // Update DOM
    if (this.dom.bandwidth) this.dom.bandwidth.textContent = `${kbPerSec} KB/s`;
    if (this.dom.cellsSec) this.dom.cellsSec.textContent = `${cellsPerSec} cells/s`;
    if (this.dom.reqSec) this.dom.reqSec.textContent = `${reqPerSec} req/s`;
    if (this.dom.latency) this.dom.latency.textContent = `${avgLatency} ms`;
    if (this.dom.modeBadge) {
      this.dom.modeBadge.textContent = this.currentMode.toUpperCase();
      this.dom.modeBadge.className = `badge mode-${this.currentMode}`;
    }
    if (this.dom.totalBytes) {
      this.dom.totalBytes.textContent = `${(this.totalBytesReceived / 1024 / 1024).toFixed(2)} MB`;
    }
    if (this.dom.totalCells) {
      this.dom.totalCells.textContent = this.totalCellsUpdated.toLocaleString();
    }
  }
}
