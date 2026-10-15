import {build} from 'esbuild';

/** Wrap generated bindings without shared WASM globals or runtime eval. */
export async function bindingFactory(entryPoint) {
  const result = await build({entryPoints:[entryPoint], bundle:true, format:'iife',
    globalName:'HyperBindings', write:false, target:'es2022',
    // SDK initialization always supplies a compiled WebAssembly.Module.
    define:{'import.meta.url':'""'}, logLevel:'silent'});
  return `export function createBindings() {\n${result.outputFiles[0].text}\nreturn HyperBindings;\n}\n`;
}
