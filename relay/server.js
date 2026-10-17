const http = require('http');
const fs = require('fs');
const path = require('path');
const url = require('url');

const PORT = process.env.PORT || 8080;
const AUTH_TOKEN = process.env.ROGP_AUTH_TOKEN || 'rogp-secret-token';
const TTL_MS = 4000; // 4 seconds TTL for frame and input packets to prevent OOM
const VIEWER_DIR = path.resolve(__dirname, '../viewer');

/**
 * In-memory state for hosts
 * hosts[hostId] = {
 *   lastSeen: number,
 *   token: string,
 *   fullSyncRequested: boolean,
 *   latestScreen: {
 *     mode: 'json' | 'binary',
 *     data: Buffer | Object,
 *     timestamp: number,
 *     cellCount: number,
 *     isFullSync: boolean
 *   } | null,
 *   // Queue of inputs from viewer to host
 *   inputQueue: Array<{
 *     seqId: number,
 *     type: string,
 *     timestamp: number,
 *     data: Object
 *   }>,
 *   stats: {
 *     screenUpdatesTotal: number,
 *     inputsTotal: number,
 *     bytesReceived: number,
 *     bytesSent: number
 *   }
 * }
 */
const hosts = new Map();

function getOrCreateHost(hostId) {
  if (!hosts.has(hostId)) {
    hosts.set(hostId, {
      lastSeen: Date.now(),
      fullSyncRequested: true, // request full sync on fresh connection
      clientConfig: null, // Viewer requested config for Host
      isBroadcasting: true,
      latestScreen: null,
      screenQueue: [],
      inputQueue: [],
      processedSeqIds: new Set(),
      stats: {
        screenUpdatesTotal: 0,
        inputsTotal: 0,
        bytesReceived: 0,
        bytesSent: 0,
        fpsEstimator: 0
      }
    });
  }
  const host = hosts.get(hostId);
  host.lastSeen = Date.now();
  return host;
}

// Memory safety & TTL eviction job running every 1000ms
setInterval(() => {
  const now = Date.now();
  for (const [hostId, host] of hosts.entries()) {
    // Drop inputs older than TTL
    host.inputQueue = host.inputQueue.filter(item => (now - item.timestamp) < TTL_MS);
    
    // Drop latest screen if older than TTL
    if (host.latestScreen && (now - host.latestScreen.timestamp) > TTL_MS) {
      host.latestScreen = null;
    }

    // Trim processed sequence IDs to prevent memory buildup
    if (host.processedSeqIds.size > 2000) {
      host.processedSeqIds.clear();
    }

    // Evict completely dead hosts after 60s
    if (now - host.lastSeen > 60000) {
      hosts.delete(hostId);
    }
  }
}, 1000);

// Helper to parse request body
function readRequestBody(req) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    let size = 0;
    const maxSizeBytes = 25 * 1024 * 1024; // 25MB max frame payload

    req.on('data', chunk => {
      size += chunk.length;
      if (size > maxSizeBytes) {
        req.destroy();
        reject(new Error('Payload too large'));
        return;
      }
      chunks.push(chunk);
    });

    req.on('end', () => {
      resolve(Buffer.concat(chunks));
    });

    req.on('error', err => reject(err));
  });
}

// Authentication validator
function checkAuth(req) {
  const authHeader = req.headers['authorization'] || req.headers['x-rogp-token'];
  if (!authHeader) return true; // Permissive default unless token explicitly required
  const token = authHeader.replace(/^Bearer\s+/i, '');
  return token === AUTH_TOKEN;
}

// Static MIME map
const MIME_TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'application/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json',
  '.webp': 'image/webp',
  '.png': 'image/png',
  '.ico': 'image/x-icon',
  '.svg': 'image/svg+xml'
};

function serveStatic(req, res, pathname) {
  let safePath = pathname === '/' ? '/index.html' : pathname;
  // Prevent directory traversal
  const resolved = path.resolve(VIEWER_DIR, '.' + safePath);
  if (!resolved.startsWith(VIEWER_DIR)) {
    res.writeHead(403, { 'Content-Type': 'text/plain' });
    res.end('Access Denied');
    return;
  }

  fs.stat(resolved, (err, stats) => {
    if (err || !stats.isFile()) {
      res.writeHead(404, { 'Content-Type': 'text/plain' });
      res.end('Not Found');
      return;
    }

    const ext = path.extname(resolved).toLowerCase();
    const contentType = MIME_TYPES[ext] || 'application/octet-stream';
    res.writeHead(200, {
      'Content-Type': contentType,
      'Cache-Control': 'no-cache, no-store, must-revalidate'
    });
    fs.createReadStream(resolved).pipe(res);
  });
}

const server = http.createServer(async (req, res) => {
  // Common CORS and security headers
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type, Authorization, X-Rogp-Token, X-Payload-Mode');
  res.setHeader('Access-Control-Expose-Headers', 'X-Payload-Mode, X-Screen-Timestamp, X-Host-Broadcasting');

  if (req.method === 'OPTIONS') {
    res.writeHead(204);
    res.end();
    return;
  }

  const parsedUrl = url.parse(req.url, true);
  const pathname = parsedUrl.pathname;

  try {
    // 1. Host Agent sends Screen updates (POST /api/host/:hostId/screen)
    const hostScreenMatch = pathname.match(/^\/api\/host\/([a-zA-Z0-9_-]+)\/screen$/);
    if (req.method === 'POST' && hostScreenMatch) {
      const hostId = hostScreenMatch[1];
      const host = getOrCreateHost(hostId);
      const rawBody = await readRequestBody(req);
      const contentType = req.headers['content-type'] || '';
      const isBinary = contentType.includes('application/octet-stream') || req.headers['x-payload-mode'] === 'binary';

      host.stats.bytesReceived += rawBody.length;
      host.stats.screenUpdatesTotal++;

      if (isBinary) {
        // Mode B: Binary Packet
        host.latestScreen = {
          mode: 'binary',
          data: rawBody,
          timestamp: Date.now(),
          byteLength: rawBody.length
        };
      } else {
        // Mode A: JSON + Base64
        try {
          const json = JSON.parse(rawBody.toString('utf-8'));
          host.latestScreen = {
            mode: 'json',
            data: json,
            timestamp: Date.now(),
            cellCount: Array.isArray(json.cells) ? json.cells.length : 0,
            byteLength: rawBody.length
          };
        } catch (jsonErr) {
          res.writeHead(400, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ error: 'Malformed JSON screen payload' }));
          return;
        }
      }

      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({
        status: 'ok',
        fullSyncRequested: host.fullSyncRequested
      }));
      // Reset fullSync flag once acknowledged
      return;
    }

    // 2. Viewer fetches pending Screen updates (GET /api/viewer/:hostId/screen)
    const viewerScreenMatch = pathname.match(/^\/api\/viewer\/([a-zA-Z0-9_-]+)\/screen$/);
    if (req.method === 'GET' && viewerScreenMatch) {
      const hostId = viewerScreenMatch[1];
      const host = getOrCreateHost(hostId);
      const screen = host.latestScreen;

      if (!screen) {
        res.writeHead(204, {
          'X-Host-Broadcasting': host.isBroadcasting ? 'true' : 'false'
        });
        res.end();
        return;
      }

      if (screen.mode === 'binary') {
        res.writeHead(200, {
          'Content-Type': 'application/octet-stream',
          'X-Payload-Mode': 'binary',
          'X-Screen-Timestamp': String(screen.timestamp),
          'X-Host-Broadcasting': host.isBroadcasting ? 'true' : 'false'
        });
        res.end(screen.data);
      } else {
        res.writeHead(200, {
          'Content-Type': 'application/json',
          'X-Payload-Mode': 'json',
          'X-Screen-Timestamp': String(screen.timestamp),
          'X-Host-Broadcasting': host.isBroadcasting ? 'true' : 'false'
        });
        res.end(JSON.stringify(screen.data));
      }
      return;
    }

    // 3. Viewer sends Input commands (POST /api/viewer/:hostId/input)
    const viewerInputMatch = pathname.match(/^\/api\/viewer\/([a-zA-Z0-9_-]+)\/input$/);
    if (req.method === 'POST' && viewerInputMatch) {
      const hostId = viewerInputMatch[1];
      const host = getOrCreateHost(hostId);
      const rawBody = await readRequestBody(req);

      try {
        const payload = JSON.parse(rawBody.toString('utf-8'));
        const events = Array.isArray(payload) ? payload : (payload.events || [payload]);

        for (const evt of events) {
          if (typeof evt.seqId !== 'undefined') {
            // Deduplicate
            if (host.processedSeqIds.has(evt.seqId)) continue;
            host.processedSeqIds.add(evt.seqId);
          }
          host.inputQueue.push({
            seqId: evt.seqId,
            type: evt.type,
            timestamp: Date.now(),
            data: evt
          });
          host.stats.inputsTotal++;
        }

        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ status: 'ok', queued: events.length }));
      } catch (err) {
        res.writeHead(400, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: 'Malformed input JSON' }));
      }
      return;
    }

    // 4. Host polls for pending Input commands & Config (GET /api/host/:hostId/input)
    const hostInputMatch = pathname.match(/^\/api\/host\/([a-zA-Z0-9_-]+)\/input$/);
    if (req.method === 'GET' && hostInputMatch) {
      const hostId = hostInputMatch[1];
      const host = getOrCreateHost(hostId);

      // Drain queued inputs
      const pendingInputs = host.inputQueue.splice(0, host.inputQueue.length);
      const fullSync = host.fullSyncRequested;
      host.fullSyncRequested = false;

      // Drain pending configuration if any
      const clientConfig = host.clientConfig;
      host.clientConfig = null;

      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({
        fullSyncRequested: fullSync,
        config: clientConfig,
        inputs: pendingInputs.map(item => item.data)
      }));
      return;
    }

    // 5. Viewer sends Configuration to Host (POST /api/viewer/:hostId/config)
    const viewerConfigMatch = pathname.match(/^\/api\/viewer\/([a-zA-Z0-9_-]+)\/config$/);
    if (req.method === 'POST' && viewerConfigMatch) {
      const hostId = viewerConfigMatch[1];
      const host = getOrCreateHost(hostId);
      const rawBody = await readRequestBody(req);

      try {
        const cfg = JSON.parse(rawBody.toString('utf-8'));
        host.clientConfig = cfg;
        if (typeof cfg.is_running === 'boolean') {
          host.isBroadcasting = cfg.is_running;
        }
        // Trigger full sync on mode or quality change
        host.fullSyncRequested = true;
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ status: 'ok', config: host.clientConfig }));
      } catch (err) {
        res.writeHead(400, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: 'Malformed config JSON' }));
      }
      return;
    }

    // 6. Viewer requests Full Screen Sync (POST /api/viewer/:hostId/sync)
    const viewerSyncMatch = pathname.match(/^\/api\/viewer\/([a-zA-Z0-9_-]+)\/sync$/);
    if (req.method === 'POST' && viewerSyncMatch) {
      const hostId = viewerSyncMatch[1];
      const host = getOrCreateHost(hostId);
      host.fullSyncRequested = true;
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ status: 'ok', message: 'Full sync requested' }));
      return;
    }

    // 6. Host status & stats (GET /api/host/:hostId/status)
    const hostStatusMatch = pathname.match(/^\/api\/host\/([a-zA-Z0-9_-]+)\/status$/);
    if (req.method === 'GET' && hostStatusMatch) {
      const hostId = hostStatusMatch[1];
      const host = hosts.get(hostId);
      if (!host) {
        res.writeHead(404, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ status: 'offline', hostId }));
        return;
      }
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({
        status: 'online',
        hostId,
        lastSeenAgoMs: Date.now() - host.lastSeen,
        stats: host.stats,
        pendingInputs: host.inputQueue.length,
        hasScreen: !!host.latestScreen,
        screenMode: host.latestScreen ? host.latestScreen.mode : null
      }));
      return;
    }

    // 7. General Relay Health / Stats
    if (pathname === '/api/stats') {
      const hostList = [];
      for (const [id, h] of hosts.entries()) {
        hostList.push({
          id,
          online: (Date.now() - h.lastSeen) < 10000,
          lastSeenAgoMs: Date.now() - h.lastSeen,
          stats: h.stats
        });
      }
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({
        server: 'Rogp Relay Server',
        uptimeSeconds: Math.floor(process.uptime()),
        hostsCount: hosts.size,
        hosts: hostList
      }));
      return;
    }

    // 8. Serve static Viewer files
    serveStatic(req, res, pathname);

  } catch (err) {
    console.error('Request error:', err);
    res.writeHead(500, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ error: 'Internal Server Error' }));
  }
});

server.listen(PORT, '0.0.0.0', () => {
  console.log(`[Rogp Relay] Listening on http://0.0.0.0:${PORT}`);
  console.log(`[Rogp Relay] Serving Viewer from: ${VIEWER_DIR}`);
});
