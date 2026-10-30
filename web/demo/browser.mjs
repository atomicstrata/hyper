import {createHyperViewer} from '../index.mjs';

const sidecar = new URL(location.href).searchParams.get('sidecar') ?? 'ws://127.0.0.1:8765';
const status = document.querySelector('.status');
const errorBox = document.querySelector('.error');
const selectionBox = document.querySelector('.selection');
const canvas = document.querySelector('canvas');

function report(error) {
  errorBox.hidden = false;
  errorBox.textContent = error?.message ?? String(error);
}

async function pageFrame(jpeg) {
  const bytes = Uint8Array.from(atob(jpeg), (char) => char.charCodeAt(0));
  const bitmap = await createImageBitmap(new Blob([bytes], {type: 'image/jpeg'}));
  const longest = Math.max(bitmap.width, bitmap.height);
  const scale = longest > 640 ? 640 / longest : 1;
  const width = Math.max(1, Math.round(bitmap.width * scale));
  const height = Math.max(1, Math.round(bitmap.height * scale));
  const surface = new OffscreenCanvas(width, height);
  const context = surface.getContext('2d', {willReadFrequently: true});
  context.drawImage(bitmap, 0, 0, width, height);
  bitmap.close();
  const pixels = context.getImageData(0, 0, width, height).data;
  return {width, height, rgba: new Uint8Array(pixels)};
}

const viewer = await createHyperViewer({canvas, backend: new URL(location.href).searchParams.get('backend') ?? 'auto'});
document.querySelector('.backend').textContent = viewer.backend;
const fixture = await (await fetch('./pages.json')).text();
await viewer.loadJson(fixture);

const pending = new Map();
let drawing = false;
function enqueue(message) {
  pending.set(message.vertexId, message.jpeg);
  pump().catch(report);
}
async function pump() {
  if (drawing) return;
  drawing = true;
  try {
    while (pending.size) {
      const batch = [...pending];
      pending.clear();
      for (const [vertexId, jpeg] of batch) {
        const frame = await pageFrame(jpeg);
        await viewer.setPageFrame(vertexId, frame.width, frame.height, frame.rgba);
      }
    }
  } finally {
    drawing = false;
    if (pending.size) pump().catch(report);
  }
}

let desiredFocus = null;
function sendFocus(vertexId) {
  desiredFocus = vertexId;
  if (socket?.readyState === WebSocket.OPEN) {
    socket.send(JSON.stringify({type: 'focus', vertexId: desiredFocus}));
  }
}

viewer.on('selectionchange', (selection) => {
  selectionBox.textContent = JSON.stringify(selection, null, 2);
  sendFocus(selection.vertices.length === 1 ? selection.vertices[0] : null);
});
viewer.on('error', report);

status.textContent = 'Connecting to the page sidecar…';
const socket = new WebSocket(sidecar);
socket.addEventListener('open', () => {
  status.textContent = 'Sidecar connected. Waiting for page stills.';
  if (desiredFocus !== null) sendFocus(desiredFocus);
});
socket.addEventListener('message', (event) => {
  const message = JSON.parse(event.data);
  if (message.type === 'error') {
    report(Error(message.message));
    return;
  }
  if (message.type === 'still' || message.type === 'frame') {
    status.textContent = message.type === 'frame'
      ? `Streaming ${message.vertexId}`
      : `Still ready for ${message.vertexId}`;
    enqueue(message);
  }
});
socket.addEventListener('close', () => {
  status.textContent = 'Sidecar disconnected.';
});
socket.addEventListener('error', () => report(Error(`Cannot reach the page sidecar at ${sidecar}`)));
