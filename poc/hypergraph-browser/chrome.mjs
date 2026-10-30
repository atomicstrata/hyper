import puppeteer from 'puppeteer-core';

const MAC_CHROME = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const FRAME_INTERVAL_MS = 200;
const MAX_EDGE = 480;

export function chromeExecutablePath(env = process.env, platform = process.platform) {
  if (env.CHROME_PATH) return env.CHROME_PATH;
  if (platform === 'darwin') return MAC_CHROME;
  throw new Error('Set CHROME_PATH to the Chrome executable');
}

// One headless Chrome. Frames are acked immediately and forwarded at about 5 Hz.
export async function launchChromeDriver({executablePath = chromeExecutablePath()} = {}) {
  const browser = await puppeteer.launch({
    executablePath,
    headless: true,
    args: ['--headless=new', '--mute-audio', '--disable-extensions'],
  });
  const pages = new Map();
  const listeners = new Set();
  let sequence = 0;

  const emit = (targetId, jpeg) => {
    for (const listener of listeners) listener(targetId, jpeg);
  };

  return {
    onFrame(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    async createTarget(url) {
      const page = sequence === 0 ? (await browser.pages())[0] : await browser.newPage();
      sequence += 1;
      const targetId = `chrome-${sequence}`;
      const cdp = await page.createCDPSession();
      try {
        await page.goto(url, {waitUntil: 'load', timeout: 20_000});
      } catch (error) {
        if (!/timeout/i.test(error?.message ?? '')) throw error;
      }
      pages.set(targetId, {page, cdp, screencasting: false, lastSent: 0, listening: false});
      return targetId;
    },
    async bringToFront(targetId) {
      const entry = pages.get(targetId);
      await entry.page.bringToFront();
      await entry.cdp.send('Page.bringToFront');
    },
    async capture(targetId) {
      const shot = await pages.get(targetId).cdp.send('Page.captureScreenshot', {
        format: 'jpeg',
        quality: 60,
        fromSurface: true,
      });
      return Buffer.from(shot.data, 'base64');
    },
    async startScreencast(targetId) {
      const entry = pages.get(targetId);
      if (entry.screencasting) return;
      entry.screencasting = true;
      if (!entry.listening) {
        entry.listening = true;
        entry.cdp.on('Page.screencastFrame', (event) => {
          entry.cdp.send('Page.screencastFrameAck', {sessionId: event.sessionId}).catch(() => {});
          if (!entry.screencasting) return;
          const now = Date.now();
          if (now - entry.lastSent < FRAME_INTERVAL_MS) return;
          entry.lastSent = now;
          emit(targetId, Buffer.from(event.data, 'base64'));
        });
      }
      await entry.cdp.send('Page.startScreencast', {
        format: 'jpeg',
        quality: 50,
        maxWidth: MAX_EDGE,
        maxHeight: MAX_EDGE,
        everyNthFrame: 3,
      });
    },
    async stopScreencast(targetId) {
      const entry = pages.get(targetId);
      if (!entry?.screencasting) return;
      entry.screencasting = false;
      await entry.cdp.send('Page.stopScreencast').catch(() => {});
    },
    async close() {
      await browser.close();
    },
  };
}
