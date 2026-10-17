# Relay Server (A) & Viewer App (C) Specifications

## Relay Server (A) - "The Middleman"
- **Environment:** Debian Linux, served behind Cloudflare Tunnel (`app.dernekportal.com`).
- **Role:** Pure state holder and message router.
- **Data Handling:** 
  - Receive POSTs from Host (Screen Data), hold them in memory/redis temporarily.
  - Serve GETs from Viewer (Screen Data).
  - Receive POSTs from Viewer (Input Data), hold them.
  - Serve GETs from Host (Input Data).
- **Cleanup:** Drop stale frames or inputs if not fetched within a specific timeout.

## Viewer App (C) - "The Frontend"
- **UI Architecture:** HTML5 Canvas or CSS Grid (optimized to hold and update 1296 distinct 40x40 image cells).
- **State Management:** Keep track of cell versions. Update only the cells received in the latest GET request payload.
- **Input Capture:** Trap mouse movements, clicks, and keyboard inputs on the frontend, map them to the 1920x1080 coordinate space, and POST them.

### Telemetry Dashboard (MUST HAVE)
The Viewer UI MUST expose real-time metrics for debugging and performance comparison (JSON vs Binary):
- Current Bandwidth Usage (KB/s).
- Number of updated cells per second.
- Number of HTTP requests per second.
- Screen update latency (if calculable).
- Current payload mode (A or B).
