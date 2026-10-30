import assert from 'node:assert/strict';
import test from 'node:test';

import {createPageSession} from './session.mjs';
import {startPageServer} from './server.mjs';

function fakeDriver() {
  const calls = [];
  const screencasting = new Set();
  const listeners = new Set();
  let next = 0;
  return {
    calls,
    screencasting,
    onFrame(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    emitFrame(targetId, jpeg) {
      for (const listener of listeners) listener(targetId, jpeg);
    },
    async createTarget(url) {
      const targetId = `t${next++}`;
      calls.push(['create', url, targetId]);
      return targetId;
    },
    async bringToFront(targetId) {
      calls.push(['front', targetId]);
    },
    async capture(targetId) {
      calls.push(['capture', targetId]);
      return Buffer.from(`jpeg-${targetId}`);
    },
    async startScreencast(targetId) {
      calls.push(['screencast-start', targetId]);
      screencasting.add(targetId);
    },
    async stopScreencast(targetId) {
      calls.push(['screencast-stop', targetId]);
      screencasting.delete(targetId);
    },
    async close() {
      calls.push(['close']);
    },
  };
}

function onceMessage(ws) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('timed out waiting for a server message')), 1000);
    ws.addEventListener('message', (event) => {
      clearTimeout(timer);
      resolve(JSON.parse(event.data));
    }, {once: true});
  });
}

test('the server binds localhost, sends stills, and forwards one live frame', async () => {
  const driver = fakeDriver();
  const server = await startPageServer({
    session: createPageSession(),
    driver,
    vertices: [
      {id: 'example', url: 'https://example.com/'},
      {id: 'bare'},
    ],
  });
  assert.equal(server.address.address, '127.0.0.1');
  const ws = new WebSocket(server.url);
  await new Promise((resolve, reject) => {
    ws.addEventListener('open', resolve, {once: true});
    ws.addEventListener('error', reject, {once: true});
  });
  const still = await onceMessage(ws);
  assert.equal(still.type, 'still');
  assert.equal(still.vertexId, 'example');
  assert.equal(Buffer.from(still.jpeg, 'base64').toString(), 'jpeg-t0');

  ws.send(JSON.stringify({type: 'focus', vertexId: 'example'}));
  await new Promise((resolve) => setTimeout(resolve, 20));
  assert.deepEqual([...driver.screencasting], ['t0']);

  const framePromise = onceMessage(ws);
  driver.emitFrame('t0', Buffer.from('live'));
  const frame = await framePromise;
  assert.equal(frame.type, 'frame');
  assert.equal(frame.vertexId, 'example');
  assert.equal(Buffer.from(frame.jpeg, 'base64').toString(), 'live');

  ws.send(JSON.stringify({type: 'focus', vertexId: null}));
  await new Promise((resolve) => setTimeout(resolve, 20));
  assert.equal(driver.screencasting.size, 0);

  ws.close();
  await server.close();
  assert.equal(driver.calls.at(-1)[0], 'close');
});
