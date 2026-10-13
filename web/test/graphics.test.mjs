import { test } from 'node:test';
import assert from 'node:assert/strict';
import { detectGraphicsBackend } from '../src/graphics.mjs';

const browser = (gpu, webgl) => ({
  navigator: gpu === undefined ? {} : { gpu },
  document: { createElement: () => ({ getContext: () => webgl ? {} : null }) },
});
test('prefers a working WebGPU adapter', async () => {
  assert.equal(await detectGraphicsBackend(browser({ requestAdapter: async () => ({}) }, true)), 'webgpu');
});
test('uses WebGL2 without navigator.gpu', async () => {
  assert.equal(await detectGraphicsBackend(browser(undefined, true)), 'webgl2');
});
test('uses WebGL2 when WebGPU has no adapter', async () => {
  assert.equal(await detectGraphicsBackend(browser({ requestAdapter: async () => null }, true)), 'webgl2');
});
test('uses WebGL2 when WebGPU is disabled by policy', async () => {
  assert.equal(await detectGraphicsBackend(browser({ requestAdapter: async () => { throw new Error('disabled'); } }, true)), 'webgl2');
});
test('reports unavailable graphics when WebGL2 is disabled', async () => {
  assert.equal(await detectGraphicsBackend(browser(undefined, false)), null);
});

test('reports unavailable graphics when context creation throws', async () => {
  assert.equal(await detectGraphicsBackend({document: {createElement: () => ({
    getContext: () => { throw new Error('blocked'); },
  })}}), null);
});

test('falls back if the WebGPU probe never resolves', async () => {
  assert.equal(await detectGraphicsBackend(browser({requestAdapter: () => new Promise(() => {})}, true), {timeoutMs: 5}), 'webgl2');
});
