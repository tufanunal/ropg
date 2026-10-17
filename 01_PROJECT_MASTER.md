# Project Master Document & Strict Guidelines

## Overview
This project is an HTTP-based, polling-driven Remote Desktop / VNC alternative. It strictly avoids video streaming protocols, WebSockets, and WebRTC. Instead, it relies on grid-based screen diffing, transmitting only changed screen regions as WebP images via standard HTTP GET/POST requests.

## Topology
- **(A) Relay Server (Homelab):** A Debian-based central server exposed via Cloudflare Tunnel at `app.dernekportal.com`. It acts as the HTTP intermediary.
- **(Bx) Host Machines:** The target PCs (Windows and Linux) being controlled.
- **(Y) Host Agent:** A Rust-based application running on (Bx) that captures the screen, diffs the grid, and emulates inputs.
- **(C) Viewer:** The web-based frontend accessed via `app.dernekportal.com` to view the screen and send controls.

## KISS Principles (Keep It Simple, Stupid)
1. No microservice architecture in the initial release.
2. No WebSockets.
3. No video streaming architectures.
4. Screen updates are purely cell-based.
5. Selectable JSON and Binary transport modes.
6. Unified protocol for Windows and Linux hosts.
7. No unmeasured performance claims; rely on real telemetry.
8. Simplest solution first; add complexity only if absolutely necessary.

## STRICT ENFORCEMENT RULES (Antigravity MUST follow)

| Category | Rule |
| :--- | :--- |
| **Communication** | MUST use HTTP GET/POST |
| **WebSocket** | MUST NOT use WebSocket |
| **WebRTC** | MUST NOT use WebRTC |
| **Video Codecs** | MUST NOT introduce a video codec streaming architecture |
| **Microservices** | MUST NOT use a microservice architecture |
| **Message Queues** | MUST NOT introduce a complex message-queue system |
| **Screen Updates** | MUST transmit only changed grid cells during normal operation |
| **Recovery** | MUST support full synchronization and cell-version recovery |
| **Performance** | MUST expose measurable performance metrics |
| **Security** | MUST enforce authentication, authorization, and safe command handling |
