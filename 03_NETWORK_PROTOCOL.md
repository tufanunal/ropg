# Network Protocol & Polling Logic

## HTTP Polling Flows

### Viewer (C) <--> Relay (A)
- **POST (Send Input):** Viewer sends keyboard/mouse events (X, Y coordinates, click types, keycodes) as JSON/YAML.
- **GET (Fetch Screen):** Viewer requests pending screen cell updates and host status.

### Host (Y) <--> Relay (A)
- **POST (Send Screen):** Host sends only the changed WebP cells (batched) to the Relay.
- **GET (Fetch Input):** Host requests pending input commands to execute.

## Advanced Communication Features

### 1. Adaptive Polling Intervals
- The polling rate MUST NOT be strictly static.
- If the network is idle (no inputs, no screen changes), increase the delay between GET requests.
- If there are pending commands or heavy screen activity, decrease the delay for near-real-time responsiveness.

### 2. Full Sync & Keyframing
- On initial connection, or if the Viewer requests a refresh, the Host MUST perform a "Full Sync".
- A Full Sync forces the capture, encoding, and transmission of ALL 1296 cells, regardless of whether they changed.

### 3. Cell Versioning & Loss Recovery
- Every cell update MUST include a version number (e.g., `C17_v42`).
- The Viewer tracks the latest version of each cell.
- If the Viewer detects a missed version due to network drops, it requests a targeted sync for those specific cells or triggers a Full Sync.

### 4. Sequence & Control Safety
- Every input command MUST have a strict Sequence ID.
- The Host Agent MUST ignore duplicate Sequence IDs to prevent double-clicks or repeated keystrokes caused by HTTP retries.
- If the Host Agent loses connection to the Relay, it MUST immediately stop all input execution.
