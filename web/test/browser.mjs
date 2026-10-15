import {chromium} from 'playwright';
import assert from 'node:assert/strict';
import {createServer} from 'node:http';
import {readFile} from 'node:fs/promises';
import {resolve, extname} from 'node:path';
import {fileURLToPath} from 'node:url';
const root=resolve(fileURLToPath(new URL('../dist/',import.meta.url)));
const harness=`<!doctype html><style>body{margin:0;display:flex}.surface{width:500px;height:400px}canvas{width:100%;height:100%;display:block}</style><div class="surface"><canvas id="a" tabindex="0"></canvas></div><div class="surface"><canvas id="b" tabindex="0"></canvas></div><input id="input"><script type="module" src="harness.mjs"></script>`;
const server=createServer(async(req,res)=>{
  try{
    const pathname=new URL(req.url,'http://localhost').pathname;
    if(pathname==='/favicon.ico'){res.writeHead(204);res.end();return;}
    if(pathname==='/harness.mjs'){res.setHeader('Content-Type','text/javascript');res.end("import {createHyperViewer} from './index.mjs';window.createHyperViewer=createHyperViewer;");return;}
    if(pathname==='/harness.html'){
      if(req.url.includes('csp=1'))res.setHeader('Content-Security-Policy',"default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; connect-src 'self'; style-src 'unsafe-inline'");
      res.setHeader('Content-Type','text/html');res.end(harness);return;
    }
    const path=resolve(root,`.${decodeURIComponent(pathname)}`);
    if(!path.startsWith(root+'/')){res.writeHead(403);res.end();return;}
    const data=await readFile(path);
    res.setHeader('Content-Type',({'.wasm':'application/wasm','.mjs':'text/javascript','.json':'application/json','.html':'text/html','.css':'text/css'})[extname(path)]??'application/octet-stream');
    res.end(data);
  }catch{res.writeHead(404);res.end();}
});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
const base=`http://127.0.0.1:${server.address().port}`;
const browser=await chromium.launch({headless:process.env.HYPER_HEADLESS!=='0',
  executablePath:process.env.HYPER_BROWSER || undefined,
  args:['--enable-unsafe-webgpu',...(process.platform==='darwin'?['--use-angle=metal']:[])]});
const evidence={browser:browser.version(),checks:[],graphics:null};
async function page(init){
  const p=await browser.newPage({viewport:{width:1280,height:900},deviceScaleFactor:1});
  p.on('pageerror',error=>console.error('Page error:',error.message));
  p.on('console',message=>{if(message.type()==='error')console.error('Browser console:',message.text());});
  if(init)await p.addInitScript(init);
  await p.goto(base+'/harness.html');await p.waitForFunction(()=>window.createHyperViewer);return p;
}
async function check(name,fn){
  let timer;
  try {await Promise.race([fn(),new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error(`Browser check timed out: ${name}`)),90_000);})]);}
  finally {clearTimeout(timer);}
  evidence.checks.push(name);console.log(`PASS ${name}`);
}
try {
  const p=await page();
  evidence.graphics=await p.evaluate(async()=>{
    const adapter=await navigator.gpu?.requestAdapter();
    const canvas=document.createElement('canvas'), gl=canvas.getContext('webgl2');
    const debug=gl?.getExtension('WEBGL_debug_renderer_info');
    const renderer=debug?gl.getParameter(debug.UNMASKED_RENDERER_WEBGL):gl?.getParameter(gl.RENDERER);
    gl?.getExtension('WEBGL_lose_context')?.loseContext();
    return {webgpu:adapter?{...adapter.info.toJSON?.(),vendor:adapter.info.vendor,architecture:adapter.info.architecture,device:adapter.info.device,description:adapter.info.description,isFallbackAdapter:adapter.info.isFallbackAdapter}:null,webgl2:renderer};
  });
  assert.ok(evidence.graphics.webgpu,'Hardware WebGPU adapter required; no silent backend substitution');
  assert.ok(evidence.graphics.webgl2,'WebGL2 required');
  assert.ok(!/swiftshader|llvmpipe|software/i.test(JSON.stringify(evidence.graphics)),'Software graphics must be reported separately');
  assert.notEqual(evidence.graphics.webgpu.isFallbackAdapter,true);
  await check('simultaneous WebGPU and WebGL2 canvases render',async()=>{
    await p.evaluate(async()=>{
      window.a=await createHyperViewer({canvas:document.querySelector('#a'),backend:'webgpu'});
      window.b=await createHyperViewer({canvas:document.querySelector('#b'),backend:'webgl2'});
      const raw=await (await fetch('./demo/sample.json')).text();
      await Promise.all([a.loadJson(raw),b.loadJson(raw)]);
      await Promise.all([a.setSettings({layoutRunning:false}),b.setSettings({layoutRunning:false})]);
    });
    assert.deepEqual(await p.evaluate(()=>[a.backend,b.backend,a.getState().graph.vertices,b.getState().graph.vertices]),['webgpu','webgl2',5,5]);
    assert.deepEqual(await p.evaluate(()=>[a.getState().renderer.backend,b.getState().renderer.backend]),['BrowserWebGpu','Gl']);
    await p.screenshot({path:process.env.HYPER_SCREENSHOT ?? '/tmp/hyper-browser-sdk.png'});
  });
  await check('camera movement and canvas resizing are independent',async()=>{
    await p.evaluate(async()=>{const raw=JSON.stringify({version:'hypergraph.v1',vertices:[{id:'x',label:'X'},{id:'y',label:'Y'},{id:'z',label:'Z'}],hyperedges:[{id:'h',vertices:['x','y','z']}]});await a.loadJson(raw);await b.loadJson(raw);await a.frame();await b.frame();await new Promise(resolve=>{let n=40;function frame(){if(--n)requestAnimationFrame(frame);else resolve();}requestAnimationFrame(frame);});});
    const left=p.locator('canvas').nth(0);
    const beforeA=await left.screenshot(), beforeB=await p.evaluate(()=>b.getState().camera.target);
    await p.mouse.move(240,190);await p.mouse.down();await p.mouse.move(320,230,{steps:12});await p.mouse.up();await p.mouse.move(1200,700);
    await p.evaluate(()=>new Promise(resolve=>{let n=30;function frame(){if(--n)requestAnimationFrame(frame);else resolve();}requestAnimationFrame(frame);}));
    assert.notDeepEqual(await left.screenshot(),beforeA);
    assert.deepEqual(await p.evaluate(()=>b.getState().camera.target),beforeB);
    const widths=await p.evaluate(async()=>{document.querySelector('.surface').style.width='380px';await new Promise(resolve=>setTimeout(resolve,200));return [a.canvas.clientWidth,b.canvas.clientWidth,a.canvas.width,b.canvas.width];});
    assert.deepEqual(widths,[380,500,380,500]);
    await p.evaluate(async()=>{document.querySelector('.surface').style.width='500px';const raw=await(await fetch('./demo/sample.json')).text();await a.loadJson(raw);await b.loadJson(raw);});
  });
  await check('canvas shortcuts respect focus and HTML input ownership',async()=>{
    await p.locator('canvas').nth(0).focus();await p.keyboard.press('Space');
    await p.waitForFunction(()=>a.getState().settings.layoutRunning);
    assert.equal(await p.evaluate(()=>b.getState().settings.layoutRunning),false);
    await p.evaluate(()=>a.setSettings({layoutRunning:false}));
    await p.locator('#input').focus();await p.keyboard.press('Space');
    assert.equal(await p.evaluate(()=>a.getState().settings.layoutRunning),false);
    assert.equal(await p.locator('#input').inputValue(),' ');
  });
  await check('selection, projection, settings and bad data are isolated',async()=>{
    const result=await p.evaluate(async()=>{
      const raw=await(await fetch('./demo/sample.json')).json();const id=raw.vertices[0].id;
      let events=0;const off=a.on('selectionchange',()=>events++);
      await a.select({vertices:[id],hyperedges:[]});
      await a.setProjection('clique');
      const before=JSON.stringify(a.getState());
      let invalid=false,settings=false;
      try{await a.loadJson('{');}catch{invalid=true;}
      try{await a.setSettings({layoutRunning:true,opacity:2});}catch{settings=true;}
      off();return {a:a.getState(),b:b.getState(),unchanged:before===JSON.stringify(a.getState()),invalid,settings,events,id};
    });
    assert.equal(result.a.projection,'clique');assert.equal(result.b.projection,'bipartite');
    assert.deepEqual(result.a.selection.vertices,[result.id]);assert.deepEqual(result.b.selection.vertices,[]);
    assert.equal(result.invalid,true);assert.equal(result.settings,true);assert.equal(result.unchanged,true);assert.ok(result.events>=1);
  });
  await check('queued load and selection are acknowledged in order',async()=>{
    await p.evaluate(async()=>{await Promise.all([a.loadJson('{"version":"hypergraph.v1","vertices":[{"id":"queued","label":"Queued"}],"hyperedges":[]}'),a.select({vertices:['queued'],hyperedges:[]})]);});
    assert.deepEqual(await p.evaluate(()=>a.getState().selection.vertices),['queued']);
  });
  await check('HIF identities, isolated vertices, empty edges and large integers survive',async()=>{
    const result=await p.evaluate(async()=>{
      await a.loadJson('{"network-type":"undirected","nodes":[{"node":1},{"node":"1"},{"node":9007199254740993},{"node":"isolated"}],"edges":[{"edge":"empty"}],"incidences":[]}');
      await a.select({vertices:['i:1','s:1','i:9007199254740993'],hyperedges:['s:empty']});
      const state=a.getState();await a.setProjection('star');await a.setProjection('bipartite');return state;
    });
    assert.equal(result.graph.vertices,4);assert.equal(result.graph.hyperedges,1);
    assert.deepEqual(result.selection.vertices,['i:1','s:1','i:9007199254740993']);
  });
  await check('pointer picking selects a visible vertex only in its own canvas',async()=>{
    await p.evaluate(async()=>{await a.loadJson('{"version":"hypergraph.v1","vertices":[{"id":"only","label":"Only"}],"hyperedges":[]}');await a.frame();});
    await p.evaluate(()=>new Promise(resolve=>{let n=40;function frame(){if(--n)requestAnimationFrame(frame);else resolve();}requestAnimationFrame(frame);}));
    await p.waitForFunction(()=>{const c=a.getState().camera;return c.focus.every((value,i)=>Math.abs(value-c.target.focus[i])<0.001)&&Math.abs(c.radius-c.target.radius)<0.001;});
    await p.screenshot({path:'/tmp/hyper-picking.png'});
    await p.mouse.click(250,200,{delay:100});await p.waitForFunction(()=>a.getState().selection.vertices.includes('only'));
    assert.deepEqual(await p.evaluate(()=>b.getState().selection.vertices),[]);
  });
  await check('dispose, reuse the same canvas, and keep the other viewer alive',async()=>{
    const result=await p.evaluate(async()=>{
      const canvas=a.canvas;await a.dispose();
      window.a=await createHyperViewer({canvas,backend:'webgpu'});
      await a.loadJson('{"version":"hypergraph.v1","vertices":[],"hyperedges":[]}');
      await b.frame();return [a.getState().graph.vertices,b.getState().graph.vertices];
    });assert.deepEqual(result,[0,5]);
  });
  await p.evaluate(async()=>{await a.dispose();await b.dispose();});
  await check('repeated disposal removes canvas event listeners',async()=>{
    const session=await p.context().newCDPSession(p);
    for(let n=0;n<5;n++) {
      await p.evaluate(async()=>{window.v=await createHyperViewer({canvas:document.querySelector('canvas'),backend:'webgl2'});window.oldCanvas=v.canvas;await v.dispose();});
      const object=await session.send('Runtime.evaluate',{expression:'window.oldCanvas'});
      const {listeners}=await session.send('DOMDebugger.getEventListeners',{objectId:object.result.objectId});
      assert.equal(listeners.length,0,'disposed canvas must have no retained input listeners');
      await session.send('Runtime.releaseObject',{objectId:object.result.objectId});
    }
    await session.detach();
  });
  await p.close();
  for(const [name,init] of [
    ['absent WebGPU',()=>Object.defineProperty(navigator,'gpu',{value:undefined})],
    ['null adapter',()=>navigator.gpu.requestAdapter=async()=>null],
    ['rejected adapter',()=>navigator.gpu.requestAdapter=async()=>{throw Error('policy');}],
  ]) await check(`${name} loads only WebGL2`,async()=>{
    const p=await page(init),requested=[];p.on('request',r=>{if(r.url().endsWith('.wasm'))requested.push(r.url());});
    assert.equal(await p.evaluate(async()=>{window.v=await createHyperViewer({canvas:document.querySelector('#a')});return v.backend;}),'webgl2');
    assert.equal(requested.length,1);assert.ok(requested[0].includes('/webgl2/'));
    await p.evaluate(()=>v.dispose());await p.close();
  });
  await check('normal auto startup downloads only WebGPU',async()=>{
    const p=await page(),requests=[];p.on('request',r=>{if(r.url().endsWith('.wasm'))requests.push(r.url());});
    assert.equal(await p.evaluate(async()=>{window.v=await createHyperViewer({canvas:document.querySelector('#a')});return v.backend;}),'webgpu');
    assert.equal(requests.length,1);assert.ok(requests[0].includes('/webgpu/'));
    await p.evaluate(()=>v.dispose());await p.close();
  });
  await check('unavailable graphics fetch no WASM',async()=>{
    const p=await page(()=>{
      Object.defineProperty(navigator,'gpu',{value:undefined});
      const original=HTMLCanvasElement.prototype.getContext;
      HTMLCanvasElement.prototype.getContext=function(type,...args){return type==='webgl2'?null:original.call(this,type,...args);};
    }),requests=[];p.on('request',r=>{if(r.url().endsWith('.wasm'))requests.push(r.url());});
    assert.match(await p.evaluate(async()=>{try{await createHyperViewer({canvas:document.querySelector('#a')});}catch(e){return e.message;}}),/Neither/);
    assert.equal(requests.length,0);await p.close();
  });
  await check('WebGPU module failure falls back without disturbing another viewer',async()=>{
    const p=await page();await p.evaluate(async()=>{window.healthy=await createHyperViewer({canvas:document.querySelector('#b'),backend:'webgl2'});});await p.route('**/renderers/webgpu/*.mjs',route=>route.abort());
    assert.equal(await p.evaluate(async()=>{window.v=await createHyperViewer({canvas:document.querySelector('#a')});return v.backend;}),'webgl2');
    await p.evaluate(async()=>{await healthy.frame();await v.dispose();await healthy.dispose();});await p.close();
  });
  await check('asynchronous WebGPU device rejection falls back to WebGL2',async()=>{
    const p=await page(()=>{
      GPUAdapter.prototype.requestDevice=async function(){throw Error('Intentional device startup failure');};
    });
    assert.equal(await p.evaluate(async()=>{window.v=await createHyperViewer({canvas:document.querySelector('#a')});return v.backend;}),'webgl2');
    await p.evaluate(()=>v.dispose());await p.close();
  });
  await check('hung device initialization times out without retaining canvas listeners',async()=>{
    const p=await page(()=>{GPUAdapter.prototype.requestDevice=()=>new Promise(()=>{});});
    await p.evaluate(()=>window.originalCanvas=document.querySelector('#a'));
    assert.equal(await p.evaluate(async()=>{window.v=await createHyperViewer({canvas:originalCanvas});return v.backend;}),'webgl2');
    const session=await p.context().newCDPSession(p);
    const object=await session.send('Runtime.evaluate',{expression:'originalCanvas'});
    const result=await session.send('DOMDebugger.getEventListeners',{objectId:object.result.objectId});
    assert.equal(result.listeners.length,0);
    await p.evaluate(()=>v.dispose());await session.detach();await p.close();
  });
  await check('session keys restore first-loaded graph views independently',async()=>{
    const p=await page();
    const result=await p.evaluate(async()=>{
      const raw='{"version":"hypergraph.v1","meta":{"id":"session-test"},"vertices":[{"id":"saved","label":"Saved"}],"hyperedges":[]}';
      let a=await createHyperViewer({canvas:document.querySelector('#a'),sessionKey:'test-session-a',backend:'webgl2'});
      let b=await createHyperViewer({canvas:document.querySelector('#b'),sessionKey:'test-session-b',backend:'webgl2'});
      await a.loadJson(raw);await b.loadJson(raw);await a.setSettings({layoutRunning:false});await b.setSettings({layoutRunning:false});
      await a.select({vertices:['saved'],hyperedges:[]});await a.frame();
      const canvases=[a.canvas,b.canvas];await a.dispose();await b.dispose();
      a=await createHyperViewer({canvas:canvases[0],sessionKey:'test-session-a',backend:'webgl2'});
      b=await createHyperViewer({canvas:canvases[1],sessionKey:'test-session-b',backend:'webgl2'});
      await a.loadJson(raw);await b.loadJson(raw);
      const result=[a.getState().selection.vertices,b.getState().selection.vertices,Object.keys(localStorage).sort()];
      await a.dispose();await b.dispose();return result;
    });
    assert.deepEqual(result,[['saved'],[],['test-session-a','test-session-b']]);await p.close();
  });
  await check('restrictive CSP works without JavaScript eval',async()=>{
    const p=await browser.newPage();await p.goto(base+'/harness.html?csp=1');await p.waitForFunction(()=>window.createHyperViewer);
    await p.evaluate(async()=>{const v=await createHyperViewer({canvas:document.querySelector('#a'),backend:'webgl2'});await v.dispose();});await p.close();
  });
  await check('later device loss emits an error on only its own viewer',async()=>{
    const p=await page(()=>{
      const original=GPUAdapter.prototype.requestDevice;
      GPUAdapter.prototype.requestDevice=async function(...args){const device=await original.apply(this,args);window.testDevice=device;return device;};
    });
    await p.evaluate(async()=>{window.v=await createHyperViewer({canvas:document.querySelector('#a'),backend:'webgpu'});window.healthy=await createHyperViewer({canvas:document.querySelector('#b'),backend:'webgl2'});window.healthyErrors=[];healthy.on('error',error=>healthyErrors.push(error));window.deviceErrors=[];v.on('error',error=>deviceErrors.push(error));testDevice.destroy();});
    await p.waitForFunction(()=>deviceErrors.length>0);assert.match(await p.evaluate(()=>deviceErrors[0]),/device lost/i);
    await p.evaluate(async()=>{await healthy.frame();await v.dispose();await healthy.dispose();});assert.deepEqual(await p.evaluate(()=>healthyErrors),[]);await p.close();
  });
  console.log(JSON.stringify(evidence,null,2));
} finally {await browser.close();await new Promise(resolve=>server.close(resolve));}
