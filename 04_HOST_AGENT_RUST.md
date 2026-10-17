# Host Agent (Y) Specifications

## Technology Stack
- **Language:** Rust (for memory safety, speed, and low-level OS access).
- **Target OS:** Cross-platform (Windows and Linux).

## Core Responsibilities
1. **Screen Capture:** Fast capture of 1920x1080 buffer.
2. **Grid Processing:** Slice buffer into 40x40 pixel chunks (1296 total).
3. **Diff Calculation:** Hash or fast byte-compare chunks against the previous frame.
4. **Encoding:** Encode only changed chunks into WebP (respecting Grayscale/Quality settings).
5. **Network:** Communicate with Relay (A) via `reqwest` or similar HTTP client using GET/POST polling. Support both JSON and Binary payload generation.
6. **Input Emulation:** Parse incoming JSON/YAML input commands (mouse X/Y, clicks, keystrokes) and execute them at the OS level (e.g., using `enigo` or OS-specific APIs).

## Security
- Agent MUST authenticate with the Relay Server.
- Agent MUST validate incoming commands to ensure they originate from an authorized Viewer session.
