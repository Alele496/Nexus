// 6.1 verification: spawn the real `nexus agent serve` via ensureServer, then
// assert readiness + key wiring + a full ACP WebSocket handshake against it.
//
// Run: node dist-electron/scripts/test-server.js   (after `npm run build`)

import { ensureServer, DEFAULT_PORT } from '../electron/server';

const BIN =
  process.env.NEXUS_DESKTOP_BIN ??
  'nexus/target/debug/nexus.exe';
const WEB = process.env.NEXUS_WEB_DIR ?? 'nexus-web/dist';

let failures = 0;
function check(name: string, cond: boolean, extra = ''): void {
  console.log(`${cond ? 'PASS' : 'FAIL'}  ${name}${extra ? `  (${extra})` : ''}`);
  if (!cond) failures++;
}

async function main(): Promise<void> {
  const handle = await ensureServer({
    binaryPath: BIN,
    webDir: WEB,
    onLog: (l) => console.log('[agent]', l),
  });
  console.log(`server on ${handle.port}, key ${handle.key.slice(0, 8)}…`);

  check('port is a number', typeof handle.port === 'number' && handle.port > 0);
  check(
    'key is 32 hex chars',
    /^[0-9a-f]{32}$/.test(handle.key),
    handle.key,
  );
  check(
    'url embeds key',
    handle.url === `http://127.0.0.1:${handle.port}/?key=${handle.key}`,
  );
  check(
    'default port preferred when free',
    handle.port === DEFAULT_PORT,
    String(handle.port),
  );

  // Readiness already awaited by ensureServer; double-check the HTTP page.
  const res = await fetch(handle.url);
  const html = await res.text();
  check('GET / returns 200', res.status === 200, String(res.status));
  check(
    'index.html carries injected server key meta',
    html.includes(`name="nexus-server-key" content="${handle.key}"`),
  );

  // Full ACP handshake over the WebSocket, authenticated with our key.
  const ws = new WebSocket(
    `ws://127.0.0.1:${handle.port}/ws?server-key=${handle.key}`,
  );
  const open = new Promise<void>((resolve, reject) => {
    ws.onopen = () => resolve();
    ws.onerror = () => reject(new Error('ws connect failed'));
  });
  await open;

  const request = (id: number, method: string, params: unknown): Promise<any> =>
    new Promise((resolve, reject) => {
      const onMsg = (ev: MessageEvent) => {
        const msg = JSON.parse(String(ev.data));
        if (msg.id === id) {
          ws.removeEventListener('message', onMsg);
          if (msg.error) reject(new Error(msg.error.message));
          else resolve(msg.result);
        }
      };
      ws.addEventListener('message', onMsg);
      ws.send(JSON.stringify({ jsonrpc: '2.0', id, method, params }));
    });

  const init = await request(1, 'initialize', {
    protocolVersion: 1,
    clientInfo: { name: 'nexus-desktop-test', version: '0.0.0' },
    _meta: { clientType: 'nexus-desktop-test', clientVersion: '0.0.0' },
  });
  check('initialize resolves', init && typeof init === 'object');

  const auth = await request(2, 'authenticate', { methodId: 'cached_token' });
  check('authenticate resolves', auth && typeof auth === 'object');

  const created = await request(3, 'session/new', {
    cwd: process.cwd(),
    mcpServers: [],
    _meta: { clientType: 'nexus-desktop-test' },
  });
  check('session/new returns sessionId', typeof created?.sessionId === 'string');

  ws.close();

  // A wrong key must be rejected (auth is real, not a formality).
  const bad = new WebSocket(
    `ws://127.0.0.1:${handle.port}/ws?server-key=deadbeef`,
  );
  const badRejected = await new Promise<boolean>((resolve) => {
    bad.onclose = () => resolve(true);
    bad.onerror = () => resolve(true);
    bad.onopen = () => resolve(false);
  });
  check('wrong key is rejected', badRejected);

  handle.stop();
  const exited = await new Promise<boolean>((resolve) => {
    const deadline = Date.now() + 3000;
    const poll = () => {
      if (handle.child?.exitCode !== null) resolve(true);
      else if (Date.now() > deadline) resolve(false);
      else setTimeout(poll, 100);
    };
    poll();
  });
  check('child is stopped after stop()', exited);

  console.log(failures === 0 ? '\nALL PASS' : `\n${failures} FAILURES`);
  process.exit(failures === 0 ? 0 : 1);
}

main().catch((err) => {
  console.error('TEST ERROR:', err);
  process.exit(2);
});
