import {supportsWebGPU, supportsWebGL2} from './graphics.mjs';

const claimedCanvases = new WeakSet();
const canvasBackends = new WeakMap();
let nextCanvas = 0;
const asError = value => value instanceof Error ? value : new Error(String(value));
const clone = value => structuredClone(value);

/** Internal platform boundary, also used to test failures without a graphics device. */
export function createSDK(platform = {}) {
  const environment = platform.environment ?? globalThis;
  const fetchAsset = platform.fetch ?? globalThis.fetch.bind(globalThis);
  const importModule = platform.importModule ?? (url => import(url.href));
  const factories = new Map();
  const modules = new Map();
  const manifests = new Map();
  const startupTimeout = platform.startupTimeoutMs ?? 30_000;
  const commandTimeout = platform.commandTimeoutMs ?? 30_000;

  function cache(map, key, load) {
    if (!map.has(key)) {
      const promise = Promise.resolve().then(load).catch(error => { map.delete(key); throw error; });
      map.set(key, promise);
    }
    return map.get(key);
  }
  async function response(url, signal) {
    const result = await fetchAsset(url, {signal});
    if (!result.ok) throw Error(`Asset request failed: ${url} (${result.status})`);
    return result;
  }
  async function compile(url) {
    const result = await response(url);
    if (platform.compile) return platform.compile(await result.arrayBuffer());
    try { return await WebAssembly.compileStreaming(result.clone()); }
    catch { return WebAssembly.compile(await result.arrayBuffer()); }
  }

  return async function createHyperViewer(options) {
    let canvas = options?.canvas;
    if (!canvas?.isConnected || !canvas.ownerDocument || typeof canvas.getContext !== 'function') {
      throw Error('canvas must be a connected HTML canvas');
    }
    if (claimedCanvases.has(canvas)) throw Error('Canvas already belongs to a viewer');
    const requested = options.backend ?? 'auto';
    if (!['auto', 'webgpu', 'webgl2'].includes(requested)) throw Error('Unknown backend');
    if (options.sessionKey !== undefined && (typeof options.sessionKey !== 'string' || !options.sessionKey.trim())) {
      throw Error('sessionKey must be a nonempty string');
    }
    const manifestURL = new URL(options.manifestUrl ?? platform.manifestURL ?? new URL('./renderers.json', import.meta.url), environment.document.baseURI);
    claimedCanvases.add(canvas);
    const originalId = canvas.id;
    const originalTabIndex = canvas.tabIndex;
    let id;
    do { id = `hyper-canvas-${++nextCanvas}`; } while (canvas.ownerDocument.getElementById?.(id));
    canvas.id = id;
    if (canvas.tabIndex < 0) canvas.tabIndex = 0;
    let backend, runtime, state, disposed = false, disposal;
    let nextCommand = 0;
    let activeAttempt;
    const pending = new Map();
    const listeners = new Map(['selectionchange', 'statechange', 'error'].map(type => [type, new Set()]));
    const notify = (type, value) => {
      for (const callback of [...listeners.get(type)]) {
        try { callback(clone(value)); } catch (error) { console.error('Hyper event listener failed', error); }
      }
    };
    function rejectPending(error) {
      for (const request of pending.values()) { clearTimeout(request.timer); request.reject(error); }
      pending.clear();
    }
    function releaseCanvas() {
      claimedCanvases.delete(canvas);
      canvas.id = originalId;
      canvas.tabIndex = originalTabIndex;
    }
    function replaceCanvas() {
      const previous = canvas;
      canvas = canvas.cloneNode(false);
      canvas.id = id;
      if (canvas.tabIndex < 0) canvas.tabIndex = 0;
      claimedCanvases.delete(previous); claimedCanvases.add(canvas);
      previous.replaceWith(canvas);
    }
    async function stopAttempt(attempt) {
      if (!attempt) return;
      attempt.stopping = true;
      try {
        attempt.runtime?.stop();
        if (attempt.runtime) await Promise.race([
          attempt.stopped,
          new Promise((_, reject) => { attempt.stopTimer = setTimeout(() => reject(Error('Viewer disposal timed out')), commandTimeout); }),
        ]);
      } finally {
        clearTimeout(attempt.stopTimer);
        attempt.runtime?.free();
        attempt.runtime = null;
      }
    }
    async function startBackend(name, manifest) {
      if (canvasBackends.has(canvas) && canvasBackends.get(canvas) !== name) replaceCanvas();
      const artifact = manifest[name];
      if (!artifact?.module || !artifact?.wasm) throw Error(`Missing ${name} build`);
      const moduleURL = new URL(artifact.module, manifestURL);
      const wasmURL = new URL(artifact.wasm, manifestURL);
      let readyResolve, readyReject, stoppedResolve;
      const ready = new Promise((resolve, reject) => { readyResolve = resolve; readyReject = reject; });
      // Observe rejection immediately, even while imports or WASM compilation are pending.
      ready.catch(() => {});
      const attempt = {stopping:false, runtime:null,
        stopped:new Promise(resolve => { stoppedResolve = resolve; })};
      activeAttempt = attempt;
      const callback = raw => {
        if (attempt !== activeAttempt) return;
        const message = JSON.parse(raw);
        if (message.type === 'disposed') { stoppedResolve(); return; }
        if (attempt.stopping) return;
        if (message.type === 'error') {
          const error = asError(message.error);
          readyReject(error);
          if (runtime === attempt.runtime && state) { rejectPending(error); notify('error', error.message); }
        } else if (message.type === 'ready') {
          state = message.state; readyResolve();
        } else if (message.type === 'ack') {
          const request = pending.get(message.id);
          if (!request) return;
          clearTimeout(request.timer); pending.delete(message.id);
          if (message.error) request.reject(asError(message.error));
          else { state = message.state; request.resolve(); }
        } else if (message.type === 'selectionchange') {
          if (state) state = {...state, selection:message.selection};
          notify('selectionchange', message.selection);
        } else if (message.type === 'statechange') {
          state = message.state; notify('statechange', state);
        }
      };
      let timer;
      const initialize = async () => {
        const [factory, compiled] = await Promise.all([
          cache(factories, moduleURL.href, () => importModule(moduleURL)),
          cache(modules, wasmURL.href, () => compile(wasmURL)),
        ]);
        if (attempt.stopping) return;
        const bindings = factory.createBindings();
        await bindings.default({module_or_path:compiled});
        if (attempt.stopping) return;
        canvasBackends.set(canvas, name);
        attempt.runtime = bindings.start(`#${id}`, callback, options.sessionKey);
        await ready;
        return attempt.runtime;
      };
      try {
        return await Promise.race([initialize(), new Promise((_, reject) => {
          timer = setTimeout(() => reject(Error(`${name} startup timed out`)), startupTimeout);
        })]);
      } catch (failure) {
        const error = asError(failure);
        await stopAttempt(attempt).catch(cleanupError => { error.cleanupError = cleanupError; });
        throw error;
      } finally { clearTimeout(timer); }
    }
    try {
      const gpu = requested !== 'webgl2' && await supportsWebGPU(environment);
      if (requested === 'webgpu' && !gpu) throw Error('WebGPU adapter unavailable');
      backend = requested === 'auto' ? (gpu ? 'webgpu' : 'webgl2') : requested;
      if (backend === 'webgl2' && !supportsWebGL2(environment)) throw Error('Neither WebGPU nor WebGL2 is available');
      const manifest = await cache(manifests, manifestURL.href, async () => {
        const controller = new AbortController();
        let timer;
        try {
          return await Promise.race([
            response(manifestURL, controller.signal).then(result => result.json()),
            new Promise((_, reject) => { timer = setTimeout(() => reject(Error('Renderer manifest startup timed out')), startupTimeout); }),
          ]);
        } finally { clearTimeout(timer); controller.abort(); }
      });
      try { runtime = await startBackend(backend, manifest); }
      catch (error) {
        if (requested !== 'auto' || backend !== 'webgpu' || !supportsWebGL2(environment)) throw error;
        replaceCanvas();
        backend = 'webgl2'; runtime = await startBackend(backend, manifest);
      }
    } catch (error) { releaseCanvas(); throw error; }

    function command(value) {
      if (disposed) return Promise.reject(Error('Viewer disposed'));
      const id = ++nextCommand;
      return new Promise((resolve, reject) => {
        const timer = setTimeout(() => { pending.delete(id); reject(Error('Viewer command timed out')); }, commandTimeout);
        pending.set(id, {resolve, reject, timer});
        try { runtime.send(id, JSON.stringify(value)); }
        catch (error) { clearTimeout(timer); pending.delete(id); reject(asError(error)); }
      });
    }
    return {
      get canvas() { return canvas; }, get backend() { return backend; },
      loadJson(json) { return typeof json === 'string' ? command({type:'loadJson',json}) : Promise.reject(Error('loadJson requires a JSON string')); },
      setProjection(projection) { return command({type:'setProjection',projection}); },
      select(selection) { return command({type:'select',selection}); },
      clearSelection() { return command({type:'clearSelection'}); },
      frame() { return command({type:'frame'}); },
      setSettings(settings) {
        if (!settings || typeof settings !== 'object' || Array.isArray(settings)) return Promise.reject(Error('Settings must be an object'));
        if (Object.values(settings).some(value => value === null || (typeof value === 'number' && !Number.isFinite(value)))) {
          return Promise.reject(Error('Settings values must be non-null and numbers must be finite'));
        }
        return command({type:'setSettings',settings});
      },
      getState() { if (disposed) throw Error('Viewer disposed'); return clone(state); },
      on(type, callback) {
        if (disposed) throw Error('Viewer disposed');
        if (!listeners.has(type) || typeof callback !== 'function') throw Error('Invalid event subscription');
        listeners.get(type).add(callback); return () => listeners.get(type).delete(callback);
      },
      dispose() {
        if (disposal) return disposal;
        disposed = true; rejectPending(Error('Viewer disposed'));
        disposal = stopAttempt(activeAttempt).finally(() => {
          runtime = null; state = null; activeAttempt = null;
          for (const set of listeners.values()) set.clear();
          releaseCanvas();
        });
        return disposal;
      },
    };
  };
}
