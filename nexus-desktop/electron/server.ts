// Nexus agent server lifecycle for the desktop shell.
//
// The shell spawns its own `nexus agent serve` child so it always knows the
// server key (generated here, passed via --secret). Port strategy: prefer the
// default 2419; if a server already occupies it, pick a free ephemeral port
// instead. A server we did not start is never attached to and never killed.
//
// Pure Node (no `electron` imports) so this module runs in plain-node tests.

import { execFile, spawn, type ChildProcess } from 'node:child_process';
import net from 'node:net';
import crypto from 'node:crypto';
import http from 'node:http';

export const DEFAULT_PORT = 2419;
export const READY_TIMEOUT_MS = 30_000;
const READY_POLL_MS = 200;

export interface ServerOptions {
  binaryPath: string;
  /** Directory containing index.html; passed to the child as $NEXUS_WEB_DIR. */
  webDir?: string;
  onLog?: (line: string) => void;
}

export interface ServerHandle {
  port: number;
  key: string;
  /** Ready-to-load app URL including the key. */
  url: string;
  child: ChildProcess | null;
  /** Kill the child (no-op if it already exited). */
  stop: () => void;
}

function isPortFree(port: number): Promise<boolean> {
  return new Promise((resolve) => {
    const sock = net.connect({ host: '127.0.0.1', port, timeout: 500 });
    sock.once('connect', () => {
      sock.destroy();
      resolve(false);
    });
    sock.once('timeout', () => {
      sock.destroy();
      resolve(true);
    });
    sock.once('error', () => resolve(true));
  });
}

function freePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const srv = net.createServer();
    srv.once('error', reject);
    srv.listen(0, '127.0.0.1', () => {
      const addr = srv.address();
      if (addr && typeof addr === 'object') {
        const port = addr.port;
        srv.close(() => resolve(port));
      } else {
        srv.close(() => reject(new Error('listener returned no port')));
      }
    });
  });
}

async function choosePort(): Promise<number> {
  if (await isPortFree(DEFAULT_PORT)) return DEFAULT_PORT;
  return freePort();
}

function httpOk(port: number): Promise<boolean> {
  return new Promise((resolve) => {
    const req = http.get(
      { host: '127.0.0.1', port, path: '/', timeout: 1500 },
      (res) => {
        res.resume();
        resolve(res.statusCode === 200);
      },
    );
    req.once('timeout', () => {
      req.destroy();
      resolve(false);
    });
    req.once('error', () => resolve(false));
  });
}

async function waitForReady(
  port: number,
  child: ChildProcess,
  timeoutMs: number,
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  let tail = '';
  child.stderr?.on('data', (d: Buffer) => {
    tail = (tail + d.toString()).slice(-4000);
  });
  while (Date.now() < deadline) {
    if (child.exitCode !== null) {
      throw new Error(
        `agent server exited early (code ${child.exitCode}): ${tail || '(no stderr)'}`,
      );
    }
    if (await httpOk(port)) return;
    await new Promise((r) => setTimeout(r, READY_POLL_MS));
  }
  throw new Error(
    `agent server not ready within ${timeoutMs}ms: ${tail || '(no stderr)'}`,
  );
}

export async function ensureServer(
  opts: ServerOptions,
): Promise<ServerHandle> {
  const key = crypto.randomBytes(16).toString('hex');
  const port = await choosePort();
  const url = `http://127.0.0.1:${port}/?key=${key}`;

  const env: Record<string, string> = { ...(process.env as Record<string, string>) };
  if (opts.webDir) env.NEXUS_WEB_DIR = opts.webDir;

  const child = spawn(
    opts.binaryPath,
    ['agent', 'serve', '--bind', `127.0.0.1:${port}`, '--secret', key],
    { env, stdio: ['ignore', 'ignore', 'pipe'], windowsHide: true },
  );
  child.stderr?.on('data', (d: Buffer) => {
    const line = d.toString().trimEnd();
    if (line) opts.onLog?.(line);
  });

  let stopped = false;
  const stop = () => {
    if (stopped) return;
    stopped = true;
    if (child.exitCode !== null) return;
    if (process.platform === 'win32') {
      // child.kill() does not terminate this console app on Windows; force-kill
      // the whole tree with taskkill instead.
      try {
        execFile('taskkill', ['/pid', String(child.pid), '/T', '/F'], () => {});
      } catch {
        child.kill();
      }
    } else {
      child.kill();
    }
  };

  try {
    await waitForReady(port, child, READY_TIMEOUT_MS);
  } catch (err) {
    stop();
    throw err;
  }
  return { port, key, url, child, stop };
}
