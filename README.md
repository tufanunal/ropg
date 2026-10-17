# Rogp

An HTTP-based, polling-driven screen streaming engine built strictly adhering to the KISS principle. It completely eliminates WebSockets, WebRTC, and video codecs in favor of a 48x27 grid diffing architecture transmitting only modified cells as WebP images.

---

## Architecture Overview

```
+--------------------+        HTTP POST / Screen        +-------------------------+
|                    | -------------------------------> |                         |
|   Host Agent (Y)   |                                  |    Relay Server (A)     |
|   (Rust / 48x27)   | <------------------------------- |    (Debian / Node.js)   |
|                    |        HTTP GET / Input          |                         |
+--------------------+                                  +-------------------------+
                                                                     ^
                                                                     |  HTTP GET / Screen
                                                                     |  HTTP POST / Input
                                                                     v
                                                        +-------------------------+
                                                        |                         |
                                                        |       Viewer (C)        |
                                                        |  (HTML5 Canvas Blitter) |
                                                        |                         |
                                                        +-------------------------+
```

### Key Specifications
- **Fixed FHD Resolution:** 1920x1080 pixels.
- **Grid Layout:** 48 columns (40px) &times; 27 rows (40px) = 1,296 total cells.
- **Addressing:** Row (`0`, `A`–`Z`) + Column (`01`–`48`), e.g. `001`, `C17`, `Z48`.
- **Diffing Engine:** Fast 64-bit FNV-1a hashing per cell; unchanged cells are never re-encoded or transmitted.
- **Payload Modes:**
  - **Mode A (JSON + Base64):** Standard JSON object containing an array of cell coordinates, versions, and Base64-encoded WebP data.
  - **Mode B (Binary Packet):** Compact custom binary format (`ROGP` magic, headers, raw WebP bytes) eliminating Base64 overhead.
- **Smart Mouse & Synthetic Cursor:**
  - Clicks and scroll events dispatch immediately.
  - Dragging is throttled to 50–80ms intervals.
  - Hover movements are debounced (200ms rest threshold).
  - Client-side synthetic cursor prevents dirtying screen cells.
- **Relay Memory Safety:** Sliding 4-second TTL eviction prevents memory buildup and OOM crashes.

---

## Directory Structure

- `shared/`: [protocol.js](file:///home/tufan/Documents/Rogp/shared/protocol.js) - Shared constants, grid coordinate formulas, cell rects.
- `relay/`: [server.js](file:///home/tufan/Documents/Rogp/relay/server.js) - HTTP Relay intermediary with in-memory buffer, TTL eviction, and static Viewer hosting.
- `host-agent/`: Rust-based agent (`main.rs`, `grid.rs`, `encoder.rs`, `packet.rs`, `input.rs`, `capture.rs`, `client.rs`).
- `viewer/`: Web-based Viewer (`index.html`, `canvas.js`, `smart_mouse.js`, `telemetry.js`, `protocol.js`, `style.css`).
- `tests/`: [integration_test.js](file:///home/tufan/Documents/Rogp/tests/integration_test.js) - Automated end-to-end integration test.

---

## Running the System

### 1. Start the Relay Server
```bash
node relay/server.js
```
The server will listen on `http://0.0.0.0:8080` and serve the Viewer web app.

### 2. Run the Rust Host Agent (Native Debian 12 GUI)

By default, running the Host Agent will launch the native GUI window:
```bash
cd host-agent
cargo run
```
Or with custom startup arguments:
```bash
cargo run -- --relay-url http://127.0.0.1:8080 --host-id host-1 --mode binary
```

#### GUI Features
- **Status Banner**: Real-time status (🟢 `ACTIVE & STREAMING` vs 🔴 `PAUSED`).
- **One-Click Sharing Toggle**: Pause or resume screen streaming at any time.
- **Connection Controls**: Update Host ID, Relay Server URL, and Auth Token on the fly.
- **Payload Mode Selection**: Switch between Mode B (Binary) and Mode A (JSON + Base64).
- **Quality & Grayscale Sliders**: Dynamically tweak WebP compression quality and grayscale mode for maximum bandwidth savings.
- **Input Control**: Enable or disable mouse & keyboard control with a single toggle.
- **Force Full Sync**: Re-transmit all 1,296 cells immediately.
- **Live Telemetry Card**: Real-time upload bandwidth (KB/s), dirty cell rate (cells/s), total transferred MB, and last received input event.

#### Headless Mode (CLI / Service)
If you want to run the Host Agent without a GUI (e.g. via `systemd` or SSH):
```bash
cargo run -- --headless
```

### 3. Open the Viewer
Navigate to `http://localhost:8080` in your web browser.
- **Telemetry HUD:** Displays real-time KB/s, cells/sec, HTTP req/s, and round-trip latency.
- **Controls:** Toggle between Binary and JSON mode, trigger manual Full Sync, or toggle the 48x27 grid and debug flash indicators.

---

## Running Tests

### Rust Unit Tests
```bash
cd host-agent
cargo test
```

### End-to-End Integration Test
```bash
node tests/integration_test.js
```
