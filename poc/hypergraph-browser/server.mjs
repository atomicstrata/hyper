import {createServer} from 'node:http';
import {readFile} from 'node:fs/promises';
import {pathToFileURL} from 'node:url';
import {WebSocketServer} from 'ws';

import {createPageSession} from './session.mjs';
import {chromeExecutablePath, launchChromeDriver} from './chrome.mjs';

// Sends captured page bytes to the browser demo. `still` is one JPEG per vertex.
// `frame` is the single live screencast. `focus` selects that vertex, or null to stop.
//
// Build the viewer once so this demo's WASM frame path exists:
//   npm run build --prefix web
// Then start the sidecar and the viewer:
//   node poc/hypergraph-browser/server.mjs
//   npm run serve --prefix web
// Open http://127.0.0.1:8080/demo/browser.html

export async function startPageServer({session, driver, vertices, host = '127.0.0.1', port = 0}) {
  const vertexByTarget = new Map();
  const stills = [];
  const clients = new Set();
  const created = [];
  const pages = {
    onFrame: (listener) => driver.onFrame(listener),
    createTarget: async (url) => {
      const targetId = await driver.createTarget(url);
      created.push(targetId);
      return targetId;
    },
    bringToFront: (targetId) => driver.bringToFront(targetId),
    capture: (targetId) => driver.capture(targetId),
    startScreencast: (targetId) => driver.startScreencast(targetId),
    stopScreencast: (targetId) => driver.stopScreencast(targetId),
  };

  const send = (socket, message) => {
    if (socket.readyState === socket.OPEN) socket.send(JSON.stringify(message));
  };
  const broadcast = (message) => {
    for (const socket of clients) send(socket, message);
  };

  driver.onFrame((targetId, jpeg) => {
    const vertexId = vertexByTarget.get(targetId);
    if (!vertexId) return;
    broadcast({type: 'frame', vertexId, jpeg: Buffer.from(jpeg).toString('base64')});
  });

  for (const vertex of vertices) {
    const before = created.length;
    const opened = await session.open({id: vertex.id, url: vertex.url ?? vertex.attrs?.page}, pages);
    if (opened.skipped) continue;
    const targetId = created[before];
    vertexByTarget.set(targetId, opened.vertexId);
    stills.push({
      type: 'still',
      vertexId: opened.vertexId,
      jpeg: Buffer.from(opened.jpeg).toString('base64'),
    });
  }

  const httpServer = createServer((_req, res) => {
    res.writeHead(200, {'content-type': 'text/plain; charset=utf-8'});
    res.end('hypergraph browser poc\n');
  });
  const sockets = new WebSocketServer({server: httpServer});
  sockets.on('connection', (socket) => {
    clients.add(socket);
    for (const still of stills) send(socket, still);
    socket.on('message', (raw) => {
      let message;
      try {
        message = JSON.parse(raw.toString());
      } catch {
        send(socket, {type: 'error', message: 'Malformed JSON'});
        return;
      }
      if (message?.type !== 'focus') return;
      Promise.resolve(session.focus(message.vertexId ?? null, pages)).catch((error) => {
        send(socket, {type: 'error', message: error.message});
      });
    });
    socket.on('close', () => clients.delete(socket));
  });

  await new Promise((resolve, reject) => {
    httpServer.once('error', reject);
    httpServer.listen(port, host, resolve);
  });
  const address = httpServer.address();
  return {
    address,
    url: `ws://${address.address}:${address.port}`,
    async close() {
      for (const socket of clients) socket.close();
      await new Promise((resolve) => sockets.close(resolve));
      await new Promise((resolve) => httpServer.close(resolve));
      if (driver.close) await driver.close();
    },
  };
}

const isMain = process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href;
if (isMain) {
  const fixtureUrl = process.argv[2] ? pathToFileURL(process.argv[2]) : new URL('./fixture.json', import.meta.url);
  const fixture = JSON.parse(await readFile(fixtureUrl, 'utf8'));
  const driver = await launchChromeDriver({executablePath: chromeExecutablePath()});
  const server = await startPageServer({
    session: createPageSession(),
    driver,
    vertices: fixture.vertices,
    port: Number(process.env.PORT ?? 8765),
  });
  console.log(`hypergraph browser sidecar ${server.url}`);
  const shutdown = () => {
    server.close().then(() => process.exit(0), () => process.exit(1));
  };
  process.once('SIGINT', shutdown);
  process.once('SIGTERM', shutdown);
}
