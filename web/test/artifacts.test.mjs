import {test} from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,writeFile,readFile,mkdir,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';
import {packageBackend,publish} from '../artifacts.mjs';

test('packaged backend names resolve to working independent binding factories',async()=>{
  const directory=await mkdtemp(join(tmpdir(),'hyper-package-'));
  try {
    const generated=join(directory,'generated'), dist=join(directory,'dist');await mkdir(generated);
    await writeFile(join(generated,'hyper.js'),'let value=0;export function increment(){return ++value;}');
    await writeFile(join(generated,'hyper_bg.wasm'),new Uint8Array([0,97,115,109,1,0,0,0]));
    const entry=await packageBackend(generated,dist,'webgpu');
    assert.equal(WebAssembly.validate(await readFile(join(dist,entry.wasm))),true);
    const {createBindings}=await import(pathToFileURL(join(dist,entry.module)).href);
    const a=createBindings(),b=createBindings();assert.equal(a.increment(),1);assert.equal(a.increment(),2);assert.equal(b.increment(),1);
  } finally {await rm(directory,{recursive:true,force:true});}
});
test('failed output replacement restores the previous complete distribution',async()=>{
  const directory=await mkdtemp(join(tmpdir(),'hyper-publish-'));
  try {
    const dist=join(directory,'dist');await mkdir(dist);await writeFile(join(dist,'index.mjs'),'old complete output');
    await assert.rejects(publish(join(directory,'missing'),dist),/ENOENT/);
    assert.equal(await readFile(join(dist,'index.mjs'),'utf8'),'old complete output');
    const next=join(directory,'next');await mkdir(next);await writeFile(join(next,'index.mjs'),'new complete output');
    await publish(next,dist);assert.equal(await readFile(join(dist,'index.mjs'),'utf8'),'new complete output');
  } finally {await rm(directory,{recursive:true,force:true});}
});
