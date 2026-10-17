# Edge Cases & Optimizations

## 1. Smart Mouse & Input Optimization
To prevent network flooding, pure mouse movements MUST NOT trigger continuous POST requests. The Viewer (C) MUST implement the following "Smart Mouse" logic:
- **Clicks & Scrolls:** MUST be dispatched immediately.
- **Dragging (Movement with LMB/RMB held down):** MUST be batched and sent at a throttled interval (e.g., 50ms - 100ms) to allow smooth window dragging.
- **Keyless Movement (Hover):** MUST be ignored during transit. The Viewer MUST wait until the mouse stops moving for a specific threshold (e.g., 200ms) before sending a single POST with the final X,Y coordinates. This ensures UI hover states (tooltips, dropdowns) work without spamming the network.

## 2. Hardware Cursor Exclusion
- When the Host Agent (Y) captures the screen, it MUST exclude the hardware cursor/mouse pointer from the capture buffer.
- **Why:** If the physical cursor is captured, every mouse movement alters the underlying grid cells, triggering unnecessary WebP encoding and transmission.
- The Viewer (C) MUST render its own synthetic cursor based on the local mouse position or the last known coordinates.

## 3. Relay Server (A) Memory Safety & TTL
- The Relay Server acts as a temporary buffer. It MUST NOT store packets indefinitely if a client disconnects unexpectedly.
- **TTL Enforcement:** All packets (Screen updates from Host, Input commands from Viewer) MUST have a strict Time-To-Live (TTL) (e.g., 3 to 5 seconds).
- The Relay Server MUST silently drop any payload that is not fetched via GET within the TTL window to prevent Out of Memory (OOM) crashes.

## 4. OS Scaling & DPI Awareness
- The Host Agent (Y) MUST be explicitly configured as "DPI-Aware" on both Windows and Linux.
- **Why:** If the OS uses display scaling (e.g., 125% or 150%), standard capture APIs might return scaled resolutions (e.g., 1536x864) instead of the raw physical resolution.
- The Agent MUST always capture the exact physical 1920x1080 pixel buffer to ensure the 48x27 grid math (40x40 pixel cells) remains mathematically perfect and uncorrupted.
