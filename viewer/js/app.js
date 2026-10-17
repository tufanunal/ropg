/**
 * Rogp Main Viewer Controller
 * Orchestrates Canvas blitting, Smart Input dispatch, and Adaptive HTTP Polling.
 */

import { ScreenCanvas } from './canvas.js';
import { SmartInputHandler } from './smart_mouse.js';
import { TelemetryMonitor } from './telemetry.js';
import { decodeBinaryPacket, decodeJsonPacket } from './protocol.js';

class RogpViewerApp {
  constructor() {
    this.hostId = 'host-1';
    this.relayUrl = window.location.origin;
    this.isRunning = false;
    this.preferredMode = 'binary'; // 'binary' or 'json'

    // Adaptive polling configuration
    this.minPollIntervalMs = 80;
    this.maxPollIntervalMs = 1200;
    this.currentPollIntervalMs = 150;
    this.consecutiveEmptyPolls = 0;
    this.pollTimeoutId = null;

    // Components
    const mainCanvas = document.getElementById('screen-canvas');
    const overlayCanvas = document.getElementById('overlay-canvas');
    const viewport = document.getElementById('viewport');
    const cursor = document.getElementById('synthetic-cursor');

    this.canvasEngine = new ScreenCanvas(mainCanvas, overlayCanvas);
    this.telemetry = new TelemetryMonitor();
    this.inputHandler = new SmartInputHandler(viewport, cursor, (inputEvent) => this.sendInput(inputEvent));

    this.initControls();
    this.startPolling();
  }

  initControls() {
    const hostInput = document.getElementById('host-id-input');
    const connectBtn = document.getElementById('connect-btn');
    const fullSyncBtn = document.getElementById('full-sync-btn');
    const toggleGridBtn = document.getElementById('toggle-grid-btn');
    const toggleFlashBtn = document.getElementById('toggle-flash-btn');
    const modeSelect = document.getElementById('mode-select');

    if (hostInput) {
      this.hostId = hostInput.value.trim() || 'host-1';
      hostInput.addEventListener('change', () => {
        this.hostId = hostInput.value.trim() || 'host-1';
        this.requestFullSync();
      });
    }

    if (connectBtn) {
      connectBtn.addEventListener('click', () => {
        if (this.isRunning) {
          this.stopPolling();
          connectBtn.textContent = 'Connect';
          connectBtn.classList.remove('btn-danger');
          connectBtn.classList.add('btn-primary');
        } else {
          this.startPolling();
          connectBtn.textContent = 'Disconnect';
          connectBtn.classList.remove('btn-primary');
          connectBtn.classList.add('btn-danger');
        }
      });
    }

    if (fullSyncBtn) {
      fullSyncBtn.addEventListener('click', () => this.requestFullSync());
    }

    if (toggleGridBtn) {
      toggleGridBtn.addEventListener('click', () => {
        this.canvasEngine.toggleGrid(!this.canvasEngine.showGrid);
        toggleGridBtn.classList.toggle('active', this.canvasEngine.showGrid);
      });
    }

    if (toggleFlashBtn) {
      toggleFlashBtn.addEventListener('click', () => {
        this.canvasEngine.flashUpdatedCells = !this.canvasEngine.flashUpdatedCells;
        toggleFlashBtn.classList.toggle('active', this.canvasEngine.flashUpdatedCells);
      });
    }

    this.isGrayscale = false;

    if (modeSelect) {
      this.preferredMode = modeSelect.value;
      modeSelect.addEventListener('change', async () => {
        this.lastModeChangeTime = Date.now();
        this.preferredMode = modeSelect.value;
        modeSelect.blur();
        await this.sendConfig({ mode: this.preferredMode });
      });
    }

    this.isHostBroadcasting = true;
    const broadcastBtn = document.getElementById('toggle-broadcast-btn');
    if (broadcastBtn) {
      broadcastBtn.addEventListener('click', async () => {
        this.isHostBroadcasting = !this.isHostBroadcasting;
        this.updateBroadcastUi();
        await this.sendConfig({ is_running: this.isHostBroadcasting });
      });
    }

    const qualitySelect = document.getElementById('quality-select');
    if (qualitySelect) {
      qualitySelect.addEventListener('change', async () => {
        const quality = parseInt(qualitySelect.value, 10);
        qualitySelect.blur();
        await this.sendConfig({ quality });
      });
    }

    const grayscaleBtn = document.getElementById('toggle-grayscale-btn');
    if (grayscaleBtn) {
      grayscaleBtn.addEventListener('click', async () => {
        this.isGrayscale = !this.isGrayscale;
        grayscaleBtn.classList.toggle('active', this.isGrayscale);
        await this.sendConfig({ grayscale: this.isGrayscale });
      });
    }

    // Main Dock Modal Overlay Controls
    const dockTriggerBtn = document.getElementById('main-dock-trigger');
    const dockOverlay = document.getElementById('main-dock-overlay');
    const closeDockBtn = document.getElementById('close-dock-btn');

    let triggerFadeTimer = null;

    const showTriggerBtn = () => {
      if (!dockTriggerBtn) return;
      dockTriggerBtn.classList.remove('faded');
      if (triggerFadeTimer) {
        clearTimeout(triggerFadeTimer);
        triggerFadeTimer = null;
      }
    };

    const startTriggerFadeTimer = () => {
      if (!dockTriggerBtn) return;
      if (triggerFadeTimer) clearTimeout(triggerFadeTimer);
      triggerFadeTimer = setTimeout(() => {
        // Fade to invisible after 3s of inactivity when modal is closed
        if (dockOverlay && dockOverlay.classList.contains('hidden')) {
          dockTriggerBtn.classList.add('faded');
        }
      }, 3000);
    };

    const openDock = () => {
      if (dockOverlay) dockOverlay.classList.remove('hidden');
      showTriggerBtn();
    };

    const closeDock = () => {
      if (dockOverlay) dockOverlay.classList.add('hidden');
      if (document.activeElement && typeof document.activeElement.blur === 'function') {
        document.activeElement.blur();
      }
      startTriggerFadeTimer();
    };

    if (dockTriggerBtn) {
      dockTriggerBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        if (dockOverlay && dockOverlay.classList.contains('hidden')) {
          openDock();
        } else {
          closeDock();
        }
      });

      dockTriggerBtn.addEventListener('mouseenter', () => {
        showTriggerBtn();
      });

      dockTriggerBtn.addEventListener('mouseleave', () => {
        startTriggerFadeTimer();
      });

      // Start initial 3s inactivity countdown
      startTriggerFadeTimer();
    }

    // Wake up button when user cursor moves near the top-center area where the button was
    window.addEventListener('mousemove', (e) => {
      const centerX = window.innerWidth / 2;
      const isOverButtonZone = e.clientY <= 60 && Math.abs(e.clientX - centerX) <= 160;
      if (isOverButtonZone) {
        showTriggerBtn();
      } else if (dockTriggerBtn && !dockTriggerBtn.classList.contains('faded') && !triggerFadeTimer) {
        startTriggerFadeTimer();
      }
    });

    if (closeDockBtn) {
      closeDockBtn.addEventListener('click', () => closeDock());
    }

    if (dockOverlay) {
      dockOverlay.addEventListener('click', (e) => {
        if (e.target === dockOverlay) {
          closeDock();
        }
      });
    }

    window.addEventListener('keydown', (e) => {
      if (e.key === 'Escape' && dockOverlay && !dockOverlay.classList.contains('hidden')) {
        closeDock();
      }
    });

    // Telemetry Modal HUD & Opacity Slider Controls
    const telemetryModal = document.getElementById('telemetry-modal');
    const closeTelemetryBtn = document.getElementById('close-telemetry-btn');
    const toggleTelemetryCheckbox = document.getElementById('toggle-telemetry-checkbox');
    const telemetrySlider = document.getElementById('telemetry-opacity-slider');
    const telemetryOpacityVal = document.getElementById('telemetry-opacity-val');

    // Restore saved opacity preference
    const savedOpacity = localStorage.getItem('rogp_telemetry_opacity') || '85';
    if (telemetrySlider) {
      telemetrySlider.value = savedOpacity;
    }
    if (telemetryOpacityVal) {
      telemetryOpacityVal.textContent = `${savedOpacity}%`;
    }
    document.documentElement.style.setProperty('--telemetry-opacity', (parseInt(savedOpacity, 10) / 100).toString());

    if (telemetrySlider) {
      telemetrySlider.addEventListener('input', (e) => {
        const val = e.target.value;
        if (telemetryOpacityVal) telemetryOpacityVal.textContent = `${val}%`;
        const decimalVal = (parseInt(val, 10) / 100).toString();
        document.documentElement.style.setProperty('--telemetry-opacity', decimalVal);
        localStorage.setItem('rogp_telemetry_opacity', val);
      });
    }

    if (toggleTelemetryCheckbox) {
      toggleTelemetryCheckbox.addEventListener('change', (e) => {
        if (telemetryModal) {
          telemetryModal.classList.toggle('hidden', !e.target.checked);
        }
      });
    }

    if (closeTelemetryBtn) {
      closeTelemetryBtn.addEventListener('click', () => {
        if (telemetryModal) telemetryModal.classList.add('hidden');
        if (toggleTelemetryCheckbox) toggleTelemetryCheckbox.checked = false;
      });
    }
  }

  updateBroadcastUi() {
    const broadcastBtn = document.getElementById('toggle-broadcast-btn');
    const hostStatusBadge = document.getElementById('stat-host-status');

    if (broadcastBtn) {
      if (this.isHostBroadcasting) {
        broadcastBtn.textContent = '🟢 Broadcast: ON';
        broadcastBtn.className = 'btn btn-broadcast-on';
        broadcastBtn.title = 'Click to Pause Host Screen Broadcasting';
      } else {
        broadcastBtn.textContent = '🔴 Broadcast: PAUSED';
        broadcastBtn.className = 'btn btn-broadcast-off';
        broadcastBtn.title = 'Click to Resume Host Screen Broadcasting';
      }
    }

    if (hostStatusBadge) {
      if (this.isHostBroadcasting) {
        hostStatusBadge.textContent = 'STREAMING';
        hostStatusBadge.className = 'badge status-broadcasting';
      } else {
        hostStatusBadge.textContent = 'PAUSED';
        hostStatusBadge.className = 'badge status-paused';
      }
    }
  }

  startPolling() {
    if (this.isRunning) return;
    this.isRunning = true;
    this.requestFullSync();
    this.scheduleNextPoll(10);
  }

  stopPolling() {
    this.isRunning = false;
    if (this.pollTimeoutId) {
      clearTimeout(this.pollTimeoutId);
      this.pollTimeoutId = null;
    }
  }

  scheduleNextPoll(delayMs) {
    if (!this.isRunning) return;
    this.pollTimeoutId = setTimeout(() => this.pollScreenUpdates(), delayMs);
  }

  async pollScreenUpdates() {
    if (!this.isRunning) return;
    const startTime = performance.now();

    try {
      const url = `${this.relayUrl}/api/viewer/${encodeURIComponent(this.hostId)}/screen`;
      const res = await fetch(url, {
        method: 'GET',
        headers: {
          'Accept': this.preferredMode === 'binary' ? 'application/octet-stream, application/json' : 'application/json'
        }
      });

      const hostBroadcastingHeader = res.headers.get('x-host-broadcasting');
      if (hostBroadcastingHeader !== null) {
        const isBroadcastingNow = hostBroadcastingHeader === 'true';
        if (isBroadcastingNow !== this.isHostBroadcasting) {
          this.isHostBroadcasting = isBroadcastingNow;
          this.updateBroadcastUi();
        }
      }

      if (res.status === 204) {
        // No screen changes
        this.consecutiveEmptyPolls++;
        this.adaptInterval(false);
        this.telemetry.recordRequest(0, 0, performance.now() - startTime, this.preferredMode);
        this.scheduleNextPoll(this.currentPollIntervalMs);
        return;
      }

      if (!res.ok) {
        this.consecutiveEmptyPolls++;
        this.adaptInterval(false);
        this.scheduleNextPoll(1000);
        return;
      }

      const contentType = res.headers.get('content-type') || '';
      const modeHeader = res.headers.get('x-payload-mode');
      const isBinary = contentType.includes('application/octet-stream') || modeHeader === 'binary';

      // Synchronize dropdown only after settling time (to avoid reverting user changes in flight)
      if (modeHeader && (!this.lastModeChangeTime || Date.now() - this.lastModeChangeTime > 2500)) {
        this.preferredMode = modeHeader;
        const modeSelect = document.getElementById('mode-select');
        if (modeSelect && modeSelect.value !== modeHeader) {
          modeSelect.value = modeHeader;
        }
      }

      let packet;
      let byteLength = 0;

      if (isBinary) {
        const arrayBuffer = await res.arrayBuffer();
        byteLength = arrayBuffer.byteLength;
        packet = await decodeBinaryPacket(arrayBuffer);
      } else {
        const text = await res.text();
        byteLength = new Blob([text]).size;
        const json = JSON.parse(text);
        packet = await decodeJsonPacket(json);
      }

      const latencyMs = Math.round(performance.now() - startTime);

      if (packet && packet.cells && packet.cells.length > 0) {
        await this.canvasEngine.applyCells(packet.cells, packet.isFullSync);
        this.consecutiveEmptyPolls = 0;
        this.adaptInterval(true);
        this.telemetry.recordRequest(byteLength, packet.cells.length, latencyMs, isBinary ? 'binary' : 'json');
      } else {
        this.consecutiveEmptyPolls++;
        this.adaptInterval(false);
        this.telemetry.recordRequest(byteLength, 0, latencyMs, isBinary ? 'binary' : 'json');
      }

    } catch (err) {
      console.warn('Poll error:', err);
      this.consecutiveEmptyPolls++;
      this.adaptInterval(false);
    }

    this.scheduleNextPoll(this.currentPollIntervalMs);
  }

  /**
   * Adaptive Polling Interval Logic
   * Speeds up to min interval on active frames, backs off gradually when idle.
   */
  adaptInterval(hasActivity) {
    if (hasActivity) {
      this.currentPollIntervalMs = this.minPollIntervalMs;
    } else {
      // Stepwise backoff
      if (this.consecutiveEmptyPolls > 15) {
        this.currentPollIntervalMs = Math.min(this.maxPollIntervalMs, this.currentPollIntervalMs + 100);
      } else if (this.consecutiveEmptyPolls > 5) {
        this.currentPollIntervalMs = Math.min(600, this.currentPollIntervalMs + 50);
      }
    }
  }

  /**
   * Send input command to Relay via HTTP POST
   */
  async sendInput(inputEvent) {
    if (!this.isRunning) return;

    try {
      const url = `${this.relayUrl}/api/viewer/${encodeURIComponent(this.hostId)}/input`;
      await fetch(url, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(inputEvent)
      });
      // Activity occurred: ramp up polling interval immediately
      this.currentPollIntervalMs = this.minPollIntervalMs;
      this.consecutiveEmptyPolls = 0;
    } catch (err) {
      console.warn('Failed to send input command:', err);
    }
  }

  /**
   * Request full 1296-cell sync from Host
   */
  async requestFullSync() {
    try {
      const url = `${this.relayUrl}/api/viewer/${encodeURIComponent(this.hostId)}/sync`;
      await fetch(url, { method: 'POST' });
      this.currentPollIntervalMs = this.minPollIntervalMs;
      this.consecutiveEmptyPolls = 0;
    } catch (err) {
      console.warn('Failed to request full sync:', err);
    }
  }

  /**
   * Send configuration update to Host (Mode A/B, quality, grayscale)
   */
  async sendConfig(config) {
    try {
      const url = `${this.relayUrl}/api/viewer/${encodeURIComponent(this.hostId)}/config`;
      await fetch(url, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(config)
      });
      // Ramp up polling immediately so new format is fetched without delay
      this.currentPollIntervalMs = this.minPollIntervalMs;
      this.consecutiveEmptyPolls = 0;
    } catch (err) {
      console.warn('Failed to send config to host:', err);
    }
  }
}

window.addEventListener('DOMContentLoaded', () => {
  window.rogpApp = new RogpViewerApp();
});
