import assert from 'node:assert/strict';
import test from 'node:test';

import {chromeExecutablePath} from './chrome.mjs';

test('CHROME_PATH selects the executable', () => {
  assert.equal(chromeExecutablePath({CHROME_PATH: '/opt/chrome'}, 'linux'), '/opt/chrome');
});

test('macOS defaults to the system Chrome app', () => {
  assert.equal(
    chromeExecutablePath({}, 'darwin'),
    '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  );
});

test('other platforms require CHROME_PATH', () => {
  assert.throws(() => chromeExecutablePath({}, 'linux'), /CHROME_PATH/);
});
