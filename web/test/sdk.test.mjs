import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createSDK} from '../src/sdk.mjs';

function setup({gpu = true, fail = false, timeout = false} = {}) {
  const loaded = [], runtimes = [];
  const doc = {baseURI:'https://host.test/app/', activeElement:null,
    createElement: () => ({getContext: () => ({getExtension:()=>null})})};
  const environment = {document:doc, navigator:{gpu: gpu ? {requestAdapter:async()=>({})} : undefined}};
  const manifest = {webgpu:{module:'gpu.mjs',wasm:'gpu.wasm'},webgl2:{module:'gl.mjs',wasm:'gl.wasm'}};
  const sdk = createSDK({environment, manifestURL:new URL('https://assets.test/sdk/renderers.json'),
    fetch:async url => ({ok:true,json:async()=>manifest,arrayBuffer:async()=>new ArrayBuffer(0)}),
    compile:async()=>({}), startupTimeoutMs:20, commandTimeoutMs:30,
    importModule:async url=> {
      loaded.push(String(url));
      return {createBindings:()=>({default:async()=>{}, start:(selector, callback)=>{
        const name=String(url).includes('gpu')?'webgpu':'webgl2';
        const runtime={send(id,raw) {
          const command=JSON.parse(raw);
          queueMicrotask(()=> callback(JSON.stringify(command.json==='invalid'
            ? {type:'ack',id,error:'Invalid graph'} : {type:'ack',id,state:{selection:{vertices:[String(id)],hyperedges:[]},graph:{vertices:id}}})));
        },stop(){queueMicrotask(()=>callback(JSON.stringify({type:'disposed'})));},free(){runtime.freed=true;}};
        runtimes.push(runtime);
        if (!timeout || name==='webgl2') queueMicrotask(()=>callback(JSON.stringify(fail && name==='webgpu'
          ? {type:'error',error:'device rejected'} : {type:'ready',state:{selection:{vertices:[],hyperedges:[]},graph:{vertices:0}}})));
        return runtime;
      }})};
    }});
  function canvas() {
    const c = {ownerDocument:doc,id:'',tabIndex:-1,isConnected:true, getContext:()=>({}),
      setAttribute(){},getAttribute(){return null;},addEventListener(){},removeEventListener(){},
      cloneNode(){return canvas();},replaceWith(next){c.replacement=next;}};
    return c;
  }
  return {sdk,canvas,loaded,runtimes};
}

test('two viewers have distinct runtimes and independent command acknowledgements',async()=>{
  const s=setup(); const a=await s.sdk({canvas:s.canvas()}); const b=await s.sdk({canvas:s.canvas()});
  await a.loadJson('{}');
  assert.equal(a.getState().graph.vertices,1);
  assert.equal(b.getState().graph.vertices,0);
  assert.equal(s.runtimes.length,2);
  assert.equal(s.loaded.length,1,'binding factory module is cached');
  await a.dispose(); await b.loadJson('{}'); assert.equal(b.getState().graph.vertices,1);
  await b.dispose();
});
test('automatic startup falls back and replaces only its own canvas',async()=>{
  const s=setup({fail:true}); const c=s.canvas(); const v=await s.sdk({canvas:c});
  assert.equal(v.backend,'webgl2'); assert.equal(v.canvas,c.replacement);
  assert.equal(s.runtimes[0].freed,true); await v.dispose();
});
test('startup timeout is bounded and allows automatic fallback',async()=>{
  const s=setup({timeout:true}); const v=await s.sdk({canvas:s.canvas()});
  assert.equal(v.backend,'webgl2'); await v.dispose();
});
test('forced WebGPU fails without loading WebGL2',async()=>{
  const s=setup({fail:true}); await assert.rejects(s.sdk({canvas:s.canvas(),backend:'webgpu'}),/device rejected/);
  assert.equal(s.loaded.length,1);
});
test('invalid command rejects without changing the last acknowledged state',async()=>{
  const s=setup(); const v=await s.sdk({canvas:s.canvas()}); await v.loadJson('{}');
  await assert.rejects(v.loadJson('invalid'),/Invalid graph/); assert.equal(v.getState().graph.vertices,1); await v.dispose();
});
test('disposal is idempotent and later operations reject',async()=>{
  const s=setup(); const v=await s.sdk({canvas:s.canvas()});
  const first=v.dispose(); assert.equal(v.dispose(),first); await first;
  await assert.rejects(v.frame(),/disposed/); assert.throws(()=>v.getState(),/disposed/);
  assert.equal(s.runtimes[0].freed,true);
});
test('canvas cannot be claimed by two active viewers',async()=>{
  const s=setup(); const c=s.canvas(); const v=await s.sdk({canvas:c});
  await assert.rejects(s.sdk({canvas:c}),/already/); await v.dispose();
});

test('invalid manifest URL does not claim or alter the canvas',async()=>{
  const s=setup(); const c=s.canvas();
  await assert.rejects(s.sdk({canvas:c,manifestUrl:'http://['}),/URL/);
  assert.equal(c.id,'');assert.equal(c.tabIndex,-1);
  const v=await s.sdk({canvas:c});await v.dispose();
});
test('nonfinite settings reject the complete patch before serialization',async()=>{
  const s=setup();const v=await s.sdk({canvas:s.canvas()});
  for(const value of [NaN,Infinity,-Infinity]) {
    await assert.rejects(v.setSettings({layoutRunning:false,opacity:value}),/finite/);
    assert.equal(v.getState().graph.vertices,0);
  }
  await v.dispose();
});
test('a hanging manifest has a startup deadline and releases the canvas',async()=>{
  const doc={baseURI:'https://host.test/',createElement:()=>({getContext:()=>({})})};
  const c={id:'',tabIndex:-1,isConnected:true,ownerDocument:doc,getContext(){}};
  const sdk=createSDK({environment:{document:doc,navigator:{}},startupTimeoutMs:10,
    manifestURL:new URL('https://host.test/renderers.json'),fetch:()=>new Promise(()=>{})});
  await assert.rejects(Promise.race([sdk({canvas:c}),new Promise((_,reject)=>setTimeout(()=>reject(Error('Test deadline exceeded')),100))]),/manifest.*timed out/i);
  assert.equal(c.id,'');assert.equal(c.tabIndex,-1);
});
test('remounting a canvas with a different backend replaces its incompatible context',async()=>{
  const s=setup();const c=s.canvas();const a=await s.sdk({canvas:c,backend:'webgpu'});await a.dispose();
  const b=await s.sdk({canvas:c,backend:'webgl2'});
  assert.notEqual(b.canvas,c);assert.equal(b.canvas,c.replacement);assert.equal(b.backend,'webgl2');await b.dispose();
});
