import assert from 'node:assert/strict';
import test from 'node:test';

import {createPageSession} from './session.mjs';

function fakeDriver() {
  const calls = [];
  const screencasting = new Set();
  let next = 0;
  return {
    calls,
    screencasting,
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
  };
}

const page = (id, url = `https://example.com/${id}`) => ({id, url});

test('opening pages captures one still each and starts no screencast', async () => {
  const driver = fakeDriver();
  const session = createPageSession();
  const ids = ['a', 'b', 'c', 'd', 'e'];
  for (const id of ids) await session.open(page(id), driver);

  assert.deepEqual(
    driver.calls.map(([kind]) => kind),
    ids.flatMap(() => ['create', 'front', 'capture']),
  );
  assert.equal(driver.screencasting.size, 0);
  assert.equal(driver.calls.filter(([kind]) => kind.startsWith('screencast')).length, 0);
});

test('focusing another page leaves only that page screencasting', async () => {
  const driver = fakeDriver();
  const session = createPageSession();
  await session.open(page('a'), driver);
  await session.open(page('b'), driver);
  driver.calls.length = 0;

  await session.focus('a', driver);
  await session.focus('b', driver);

  assert.deepEqual(driver.calls, [
    ['front', 't0'],
    ['screencast-start', 't0'],
    ['screencast-stop', 't0'],
    ['front', 't1'],
    ['screencast-start', 't1'],
  ]);
  assert.deepEqual([...driver.screencasting], ['t1']);
});

test('a sixth focus still leaves one screencast', async () => {
  const driver = fakeDriver();
  const session = createPageSession();
  for (let i = 0; i < 6; i += 1) await session.open(page(String(i)), driver);
  for (let i = 0; i < 6; i += 1) await session.focus(String(i), driver);
  assert.equal(driver.screencasting.size, 1);
});

test('clearing focus stops the live screencast', async () => {
  const driver = fakeDriver();
  const session = createPageSession();
  await session.open(page('a'), driver);
  await session.focus('a', driver);
  await session.focus(null, driver);
  assert.equal(driver.screencasting.size, 0);
});

test('the 101st page target is refused', async () => {
  const driver = fakeDriver();
  const session = createPageSession();
  for (let i = 0; i < 100; i += 1) await session.open(page(String(i)), driver);
  await assert.rejects(session.open(page('overflow'), driver), /100/);
  assert.equal(driver.calls.filter(([kind]) => kind === 'create').length, 100);
});

test('a page without a url is skipped', async () => {
  const driver = fakeDriver();
  const session = createPageSession();
  const result = await session.open({id: 'bare'}, driver);
  assert.equal(result.skipped, true);
  assert.deepEqual(driver.calls, []);
});
