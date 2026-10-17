/**
 * Rogp Smart Mouse & Input Capture Engine
 * 
 * Rules:
 * 1. Clicks & Scrolls: Dispatched immediately.
 * 2. Dragging (Mouse move with button down): Throttled to 50-80ms.
 * 3. Keyless Hover (Mouse move without button down): Debounced (200ms idle threshold).
 * 4. Synthetic Cursor: Local client-side cursor rendering to prevent dirtying screen cells.
 * 5. Strict Sequence IDs to avoid duplicate execution on network retry.
 */

import { SCREEN_WIDTH, SCREEN_HEIGHT } from './protocol.js';

export class SmartInputHandler {
  constructor(viewportElement, cursorElement, sendInputCallback) {
    this.viewport = viewportElement;
    this.cursor = cursorElement;
    this.sendInput = sendInputCallback;

    this.seqId = 1;
    this.isMouseDown = false;
    this.activeButton = 0;

    this.lastX = 0;
    this.lastY = 0;

    // Dragging throttle timer
    this.lastDragSendTime = 0;
    this.dragThrottleMs = 60; // 60ms for smooth window drag

    // Hover debounce timer
    this.hoverDebounceTimer = null;
    this.hoverThresholdMs = 200; // 200ms rest threshold

    this.enabled = true;
    this.initEvents();
  }

  nextSeqId() {
    return this.seqId++;
  }

  /**
   * Scale browser viewport coordinates to exact 1920x1080 canvas coordinates
   */
  mapCoordinates(e) {
    const rect = this.viewport.getBoundingClientRect();
    const scaleX = SCREEN_WIDTH / rect.width;
    const scaleY = SCREEN_HEIGHT / rect.height;

    let x = Math.round((e.clientX - rect.left) * scaleX);
    let y = Math.round((e.clientY - rect.top) * scaleY);

    x = Math.max(0, Math.min(SCREEN_WIDTH - 1, x));
    y = Math.max(0, Math.min(SCREEN_HEIGHT - 1, y));

    return { x, y, clientX: e.clientX, clientY: e.clientY };
  }

  updateSyntheticCursor(clientX, clientY, visible = true) {
    if (!this.cursor) return;
    if (!visible) {
      this.cursor.style.display = 'none';
      return;
    }
    this.cursor.style.display = 'block';
    this.cursor.style.transform = `translate(${clientX}px, ${clientY}px)`;
  }

  initEvents() {
    const catcher = document.getElementById('mouse-catcher');
    const stage = document.getElementById('main-stage') || this.viewport;
    const targets = [catcher, this.viewport, stage].filter(Boolean);

    // Mouse Move (Drag vs Hover)
    const handleMove = (e) => {
      if (!this.enabled) return;
      const dockOverlay = document.getElementById('main-dock-overlay');
      if (dockOverlay && !dockOverlay.classList.contains('hidden')) return;

      const { x, y, clientX, clientY } = this.mapCoordinates(e);
      this.lastX = x;
      this.lastY = y;
      this.updateSyntheticCursor(clientX, clientY, true);

      if (this.isMouseDown) {
        // Dragging mode: Throttled dispatch
        const now = performance.now();
        if (now - this.lastDragSendTime >= this.dragThrottleMs) {
          this.lastDragSendTime = now;
          this.sendInput({
            seqId: this.nextSeqId(),
            type: 'mouse_drag',
            x,
            y,
            button: this.activeButton
          });
        }
      } else {
        // Keyless hover mode: Debounced dispatch
        if (this.hoverDebounceTimer) {
          clearTimeout(this.hoverDebounceTimer);
        }
        this.hoverDebounceTimer = setTimeout(() => {
          this.sendInput({
            seqId: this.nextSeqId(),
            type: 'mouse_hover',
            x: this.lastX,
            y: this.lastY
          });
        }, this.hoverThresholdMs);
      }
    };

    // Mouse Down (Immediate)
    const handleDown = (e) => {
      if (!this.enabled) return;
      const dockOverlay = document.getElementById('main-dock-overlay');
      if (dockOverlay && !dockOverlay.classList.contains('hidden')) return;

      e.preventDefault();
      this.isMouseDown = true;
      this.activeButton = e.button; // 0: left, 1: middle, 2: right
      const { x, y } = this.mapCoordinates(e);

      this.sendInput({
        seqId: this.nextSeqId(),
        type: 'mouse_down',
        x,
        y,
        button: e.button
      });
    };

    // Mouse Up (Immediate)
    const handleUp = (e) => {
      if (!this.isMouseDown || !this.enabled) return;
      this.isMouseDown = false;
      const { x, y } = this.mapCoordinates(e);

      this.sendInput({
        seqId: this.nextSeqId(),
        type: 'mouse_up',
        x,
        y,
        button: e.button
      });
    };

    // Wheel / Scroll (Immediate)
    const handleWheel = (e) => {
      if (!this.enabled) return;
      const dockOverlay = document.getElementById('main-dock-overlay');
      if (dockOverlay && !dockOverlay.classList.contains('hidden')) return;

      e.preventDefault();
      const { x, y } = this.mapCoordinates(e);

      this.sendInput({
        seqId: this.nextSeqId(),
        type: 'mouse_wheel',
        x,
        y,
        deltaX: Math.sign(e.deltaX),
        deltaY: Math.sign(e.deltaY)
      });
    };

    targets.forEach((el) => {
      el.addEventListener('mousemove', handleMove);
      el.addEventListener('mousedown', handleDown);
      el.addEventListener('wheel', handleWheel, { passive: false });
      el.addEventListener('contextmenu', (e) => e.preventDefault());
    });

    const handleMouseLeave = () => {
      this.updateSyntheticCursor(0, 0, false);
      if (this.hoverDebounceTimer) {
        clearTimeout(this.hoverDebounceTimer);
      }
    };

    if (catcher) catcher.addEventListener('mouseleave', handleMouseLeave);
    this.viewport.addEventListener('mouseleave', handleMouseLeave);

    window.addEventListener('mouseup', handleUp);

    // Safeguard: Release mouse if window loses focus
    window.addEventListener('blur', () => {
      if (this.isMouseDown) {
        this.isMouseDown = false;
        this.sendInput({
          seqId: this.nextSeqId(),
          type: 'mouse_up',
          x: this.lastX,
          y: this.lastY,
          button: this.activeButton
        });
      }
    });

    // Keyboard Events
    window.addEventListener('keydown', (e) => {
      if (!this.enabled) return;

      // Hotkey Ctrl+Alt+K toggles Session Controls modal locally - do not forward to host
      if ((e.ctrlKey || e.metaKey) && e.altKey && (e.code === 'KeyK' || e.key.toLowerCase() === 'k')) {
        return;
      }

      // Do not capture if modal is open or typing in an input field
      const dockOverlay = document.getElementById('main-dock-overlay');
      if (dockOverlay && !dockOverlay.classList.contains('hidden')) {
        return;
      }
      if (e.target.tagName === 'INPUT' || e.target.tagName === 'SELECT' || e.target.tagName === 'TEXTAREA') {
        return;
      }
      e.preventDefault();
      this.sendInput({
        seqId: this.nextSeqId(),
        type: 'key_down',
        key: e.key,
        code: e.code,
        altKey: e.altKey,
        ctrlKey: e.ctrlKey,
        shiftKey: e.shiftKey,
        metaKey: e.metaKey
      });
    });

    window.addEventListener('keyup', (e) => {
      if (!this.enabled) return;

      // Hotkey Ctrl+Alt+K toggles Session Controls modal locally - do not forward to host
      if ((e.ctrlKey || e.metaKey) && e.altKey && (e.code === 'KeyK' || e.key.toLowerCase() === 'k')) {
        return;
      }

      const dockOverlay = document.getElementById('main-dock-overlay');
      if (dockOverlay && !dockOverlay.classList.contains('hidden')) {
        return;
      }
      if (e.target.tagName === 'INPUT' || e.target.tagName === 'SELECT' || e.target.tagName === 'TEXTAREA') {
        return;
      }
      e.preventDefault();
      this.sendInput({
        seqId: this.nextSeqId(),
        type: 'key_up',
        key: e.key,
        code: e.code
      });
    });
  }
}
