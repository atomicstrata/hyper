export async function supportsWebGPU(environment = globalThis, timeoutMs = 3000) {
  let timer;
  try {
    const adapter = await Promise.race([
      environment.navigator?.gpu?.requestAdapter(),
      new Promise(resolve => { timer = setTimeout(() => resolve(null), timeoutMs); }),
    ]);
    return Boolean(adapter);
  } catch {
    return false;
  } finally {
    clearTimeout(timer);
  }
}

export function supportsWebGL2(environment = globalThis) {
  try {
    const gl = environment.document.createElement('canvas').getContext('webgl2');
    if (gl) {
      gl.getExtension?.('WEBGL_lose_context')?.loseContext();
      return true;
    }
  } catch {
    // Browser policy can reject context creation even when the API exists.
  }
  return false;
}

export async function detectGraphicsBackend(environment = globalThis, {timeoutMs = 3000} = {}) {
  if (await supportsWebGPU(environment, timeoutMs)) return 'webgpu';
  return supportsWebGL2(environment) ? 'webgl2' : null;
}
