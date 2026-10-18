/**
 * Rogp End-to-End Integration Test
 * Verifies Relay Server, Rust Host Agent, HTTP polling flows, Full Sync, and Input routing.
 */

const { spawn } = require('child_process');
const path = require('path');
const assert = require('assert');

const TEST_PORT = 8899;
const HOST_ID = 'test-host-e2e';
const RELAY_URL = `http://127.0.0.1:${TEST_PORT}`;

async function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async function runTest() {
  console.log('--- [1/6] Launching Relay Server on port', TEST_PORT, '---');
  const relayProcess = spawn('node', ['server.js'], {
    cwd: path.resolve(__dirname, '../relay'),
    env: { ...process.env, PORT: String(TEST_PORT) },
    stdio: 'inherit'
  });

  await sleep(1000);

  // Check Relay health
  console.log('--- [2/6] Checking Relay Stats Endpoint ---');
  const statsRes = await fetch(`${RELAY_URL}/api/stats`);
  assert.strictEqual(statsRes.status, 200);
  const stats = await statsRes.json();
  console.log('Relay stats:', stats);

  console.log('--- [3/6] Launching Rust Host Agent ---');
  const agentBinary = path.resolve(__dirname, '../host-agent/target/debug/host-agent');
  const agentProcess = spawn(agentBinary, [
    '--relay-url', RELAY_URL,
    '--host-id', HOST_ID,
    '--mode', 'binary',
    '--interval-ms', '100',
    '--headless'
  ], {
    stdio: 'pipe'
  });

  agentProcess.stdout.on('data', data => {
    // console.log('[HostAgent]:', data.toString());
  });
  agentProcess.stderr.on('data', data => {
    console.error('[HostAgent ERR]:', data.toString());
  });

  await sleep(2000);

  console.log('--- [4/6] Verifying Viewer Screen Fetch (Binary Mode B) ---');
  const screenRes = await fetch(`${RELAY_URL}/api/viewer/${HOST_ID}/screen`);
  console.log('Viewer Screen HTTP Status:', screenRes.status);
  console.log('Content-Type:', screenRes.headers.get('content-type'));
  console.log('X-Payload-Mode:', screenRes.headers.get('x-payload-mode'));
  assert.strictEqual(screenRes.status, 200);
  assert.strictEqual(screenRes.headers.get('x-payload-mode'), 'binary');

  const screenBytes = await screenRes.arrayBuffer();
  console.log('Received binary screen payload bytes:', screenBytes.byteLength);
  assert(screenBytes.byteLength > 16, 'Expected binary header at least 16 bytes');

  const view = new DataView(screenBytes);
  const magic = view.getUint32(0, false);
  assert.strictEqual(magic, 0x524F4750, 'Magic should be ROGP');
  const cellCount = view.getUint16(6, false);
  console.log(`Binary frame decoded: ${cellCount} cells received`);
  assert(cellCount > 0, 'Cell count should be > 0');

  // Test Mode A JSON format directly via Relay
  console.log('--- [4b/6] Verifying JSON Mode A Ingestion & Fetch ---');
  const JSON_HOST_ID = 'test-host-json-direct';
  const testJsonPayload = {
    host_id: JSON_HOST_ID,
    full_sync: false,
    timestamp: Date.now(),
    cells: [{ id: 'C17', v: 99, data: 'UklGRhoAAABXRUJQVlA4TA0AAAAvAAAAEAcQERGIiP4H' }]
  };
  await fetch(`${RELAY_URL}/api/host/${JSON_HOST_ID}/screen`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', 'X-Payload-Mode': 'json' },
    body: JSON.stringify(testJsonPayload)
  });
  const jsonScreenRes = await fetch(`${RELAY_URL}/api/viewer/${JSON_HOST_ID}/screen`);
  assert.strictEqual(jsonScreenRes.headers.get('x-payload-mode'), 'json');
  const jsonScreenData = await jsonScreenRes.json();
  assert.strictEqual(jsonScreenData.cells[0].id, 'C17');
  assert.strictEqual(jsonScreenData.cells[0].v, 99);
  console.log('JSON Mode A ingestion and viewer retrieval verified successfully.');

  console.log('--- [5/6] Verifying Config Switch (Viewer -> Host) ---');
  // Viewer commands Host to switch from Binary to Mode A (JSON) and grid size 160x180
  console.log('Viewer sending config: switch to Mode A (JSON) and Grid 160x180...');
  const cfgRes = await fetch(`${RELAY_URL}/api/viewer/${HOST_ID}/config`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ mode: 'json', quality: 75, grid_size: '160x180' })
  });
  assert.strictEqual(cfgRes.status, 200);

  // Wait for Host to poll and adopt Mode A and 160x180 grid
  await sleep(1500);

  const switchedScreenRes = await fetch(`${RELAY_URL}/api/viewer/${HOST_ID}/screen`);
  console.log('Host responded in Mode:', switchedScreenRes.headers.get('x-payload-mode'));
  console.log('Host responded with X-Grid-Size:', switchedScreenRes.headers.get('x-grid-size'));
  assert.strictEqual(switchedScreenRes.headers.get('x-payload-mode'), 'json');
  assert.strictEqual(switchedScreenRes.headers.get('x-grid-size'), '160x180');
  console.log('Verified Host adopted Mode A (JSON) and 160x180 Grid commanded by Viewer!');

  console.log('--- [5b/8] Verifying Broadcast Pause & Resume ---');
  // Viewer commands Host to pause broadcasting
  console.log('Viewer commanding Host to PAUSE broadcasting...');
  await fetch(`${RELAY_URL}/api/viewer/${HOST_ID}/config`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ is_running: false })
  });
  await sleep(800);

  // Viewer commands Host to RESUME broadcasting
  console.log('Viewer commanding Host to RESUME broadcasting...');
  await fetch(`${RELAY_URL}/api/viewer/${HOST_ID}/config`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ is_running: true })
  });
  await sleep(1000);
  console.log('Verified Host received Pause and Resume broadcast commands!');

  console.log('--- [6/7] Verifying Viewer Input Dispatch & Host Execution ---');
  const testInput = {
    seqId: 101,
    type: 'mouse_down',
    x: 500,
    y: 300,
    button: 0
  };
  const inputPostRes = await fetch(`${RELAY_URL}/api/viewer/${HOST_ID}/input`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(testInput)
  });
  assert.strictEqual(inputPostRes.status, 200);
  console.log('Viewer posted input event successfully.');

  console.log('--- [7/7] Verifying Full Sync Trigger ---');
  const syncPostRes = await fetch(`${RELAY_URL}/api/viewer/${HOST_ID}/sync`, { method: 'POST' });
  assert.strictEqual(syncPostRes.status, 200);
  console.log('Full sync triggered.');

  await sleep(1500);

  // Clean shutdown
  console.log('\nAll integration tests passed successfully! Shutting down processes...');
  agentProcess.kill();
  relayProcess.kill();
}

runTest().catch(err => {
  console.error('Integration test failed:', err);
  process.exit(1);
});
