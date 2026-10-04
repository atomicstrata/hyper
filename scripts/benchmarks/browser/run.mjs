import fs from 'node:fs/promises';
import path from 'node:path';
import http from 'node:http';
import crypto from 'node:crypto';
import os from 'node:os';
import {fileURLToPath} from 'node:url';
import {build,stop} from 'esbuild';
import {chromium} from 'playwright';

const here=path.dirname(fileURLToPath(import.meta.url)),root=path.resolve(here,'../../..');
const args=process.argv.slice(2), value=(name,fallback)=>args.includes(name)?args[args.indexOf(name)+1]:fallback;
const validateOnly=args.includes('--validate-only');
const datasets=value('--datasets',validateOnly?'correctness':'correctness,moderate,large,high-arity-10,high-arity-100,high-arity-1000,scientific,mathlib').split(',');
const tools=value('--tools','3d-force-graph,cytoscape,sigma').split(',');
const modes=value('--modes','static,camera-motion').split(',');
if(modes.some(m=>!['static','camera-motion'].includes(m)))throw Error('Modes must be static or camera-motion');
const jobTimeoutMs=Number(value('--job-timeout-ms','180000'));
if(!Number.isFinite(jobTimeoutMs)||jobTimeoutMs<=0)throw Error('Invalid watchdog duration');
const output=path.resolve(root,value('--output','target/benchmark/browser'));
const executable=value('--executable',process.env.BENCHMARK_CHROMIUM_EXECUTABLE);
const headed=!args.includes('--headless');
const seed=Number(value('--seed','20261003')),warmup=Number(value('--warmup','120')),samples=Number(value('--samples','300')),repetitions=Number(value('--repetitions','5'));
if(!validateOnly&&(warmup<120||samples<300||repetitions<5))throw Error('Protocol requires >=120 warmup frames, >=300 samples, >=5 repetitions');
await fs.mkdir(output,{recursive:true});
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
const canonical=p=>JSON.stringify(p);
function assertions(graph,scene){
 const ids=scene.nodes.map(n=>n.id);if(new Set(ids).size!==ids.length)throw Error('Duplicate scene node IDs');
 if(new Set(scene.links.map(l=>l.id)).size!==scene.links.length)throw Error('Duplicate scene link IDs');
 const expectedIds=[...graph.vertices.map(v=>'v:'+v.id),...graph.hyperedges.map(e=>'e:'+e.id)].sort();
 if(canonical([...ids].sort())!==canonical(expectedIds))throw Error('Missing vertex or edge hub; includes isolates and empty edges');
 const expected=graph.hyperedges.flatMap(e=>[...new Set(e.vertices)].map(v=>'e:'+e.id+'\0v:'+v)).sort();
 const actual=scene.links.map(l=>l.source+'\0'+l.target).sort();
 if(canonical(expected)!==canonical(actual))throw Error('Canonical exact memberships differ from incidence scene');
 for(const n of scene.nodes)if(![n.x,n.y,n.z].every(Number.isFinite))throw Error('Non-finite coordinate '+n.id);
 const sceneById=new Map(scene.nodes.map(n=>[n.id,n]));
 for(const v of graph.vertices){const n=sceneById.get('v:'+v.id);if(canonical(v.attrs.position)!==canonical([n.x,n.y,n.z]))throw Error('Vertex coordinate mismatch '+v.id);}
 const rawMemberships=graph.hyperedges.reduce((n,e)=>n+e.vertices.length,0);
 return {vertices:graph.vertices.length,hyperedges:graph.hyperedges.length,rawMemberships,uniqueMemberships:expected.length,duplicateMemberships:rawMemberships-expected.length,sceneNodes:ids.length,sceneLinks:actual.length,maxArity:Math.max(0,...graph.hyperedges.map(e=>new Set(e.vertices).size)),emptyEdges:graph.hyperedges.filter(e=>!e.vertices.length).length,exactMemberships:true};
}
function validateSnapshot(snapshot,scene){
 const nodes=new Map(snapshot.nodes.map(n=>[n.id,n])),links=new Map(snapshot.links.map(l=>[l.id,l]));
 if(nodes.size!==scene.nodes.length||links.size!==scene.links.length)throw Error('Adapter topology count mismatch');
 for(const n of scene.nodes){const a=nodes.get(n.id);if(!a||a.x!==n.x||a.y!==n.y||a.z!==n.z)throw Error('Adapter coordinate/ID mismatch '+n.id);}
 for(const l of scene.links){const a=links.get(l.id);if(!a||a.source!==l.source||a.target!==l.target)throw Error('Adapter incidence mismatch '+l.id);}
 return {nodes:nodes.size,links:links.size,exactIDsCoordinatesAndMemberships:true,topologySha256:sha(canonical({nodes:[...nodes.keys()].sort(),links:[...links.values()].sort((a,b)=>a.id.localeCompare(b.id))}))};
}
let randomState=seed>>>0;function random(){randomState=(Math.imul(1664525,randomState)+1013904223)>>>0;return randomState/2**32;}
function shuffle(a){for(let i=a.length-1;i>0;i--){const j=Math.floor(random()*(i+1));[a[i],a[j]]=[a[j],a[i]];}return a;}
const bundle=await build({entryPoints:[path.join(here,'client.js')],bundle:true,write:false,format:'iife',platform:'browser'});
const html='<!doctype html><html><head><meta charset="utf-8"><style>html,body,#view{margin:0;width:1920px;height:1080px;background:#080c14;overflow:hidden}</style></head><body><div id="view"></div><script src="/bundle.js"></script></body></html>';
const server=http.createServer((req,res)=>{res.setHeader('Content-Type',req.url==='/bundle.js'?'text/javascript':'text/html');res.end(req.url==='/bundle.js'?bundle.outputFiles[0].contents:html);});
await new Promise(r=>server.listen(0,'127.0.0.1',r));
const url='http://127.0.0.1:'+server.address().port;
const launchArgs=['--disable-background-timer-throttling','--disable-renderer-backgrounding','--disable-backgrounding-occluded-windows','--window-size=1920,1080'];
if(args.includes('--swiftshader'))launchArgs.push('--use-angle=swiftshader','--enable-unsafe-swiftshader');
const browser=await chromium.launch({headless:!headed,...(executable?{executablePath:executable}:{}),args:launchArgs});
const context=await browser.newContext({viewport:{width:1920,height:1080},deviceScaleFactor:1});
await context.addInitScript(()=>{window.__glContexts=[];const original=HTMLCanvasElement.prototype.getContext;HTMLCanvasElement.prototype.getContext=function(type,...rest){const result=original.call(this,type,...rest);if(result&&/^(webgl2?|experimental-webgl)$/.test(type)&&!window.__glContexts.includes(result))window.__glContexts.push(result);return result;};});
const packageJSON=JSON.parse(await fs.readFile(path.join(here,'package.json')));
const session=await browser.newBrowserCDPSession();
const [browserInfo,systemInfo]=await Promise.all([session.send('Browser.getVersion'),session.send('SystemInfo.getInfo')]);
const lock=await fs.readFile(path.join(here,'package-lock.json'));
const report={kind:validateOnly?'untimed-adapter-validation':'browser-frozen-incidence-benchmark',startedUTC:new Date().toISOString(),environment:{os:os.type(),release:os.release(),arch:os.arch(),cpus:os.cpus().map(c=>c.model),ramBytes:os.totalmem(),node:process.version,browser:browserInfo,gpu:systemInfo.gpu,headed,executable:executable||chromium.executablePath(),launchArgs,viewport:{width:1920,height:1080,dpr:1},packageVersions:{...packageJSON.dependencies,...packageJSON.devDependencies},lockfileSha256:sha(lock),powerThermalAndBackgroundLoad:process.env.BENCHMARK_CONDITIONS||'unrecorded; set BENCHMARK_CONDITIONS before publishing',vsync:'browser default; rAF cadence recorded, presentation unverified'},protocol:{seed,warmup,samples,repetitions,modes,jobTimeoutMs,order:'seeded Fisher-Yates over tool/dataset/mode for each repetition',gpuDuration:'not measured',memory:'Peak browser/GPU memory not measured; host total RAM is environment capacity only',inputLatency:'two-rAF scripted update surrogate, not human input or presentation latency',twoDvsThreeD:'Cytoscape and Sigma XY projection differs from 3D sphere and perspective rendering'},datasets:{},runs:[],unavailable:[{tool:'HNX Widget',reason:'Unsupported matched frozen incidence track: official 58e795d6a362dcd3bbd9ccff462052956a40f4f8 src/HypernetxWidgetView.js pos input fixes vertex parent nodes, not hyperedge hubs; public renderer runs charge/link/center/collide/bound simulation. Notebook wrapper also omits isolated vertices. See README capability probe. Native group-exploration remains a separate track'}]};
function distribution(a){const sorted=[...a].sort((x,y)=>x-y);const p=q=>sorted[Math.min(sorted.length-1,Math.floor(q*(sorted.length-1)))];return {p50Ms:p(.5),p95Ms:p(.95),p99Ms:p(.99),meanMs:a.reduce((x,y)=>x+y,0)/a.length,maxMs:sorted.at(-1)};}
const scenes=new Map();
try{
 for(const name of datasets){
  const graphPath=path.join(root,'target/benchmark/datasets',name+'.json'),scenePath=graphPath.replace(/\.json$/,'.incidence.json');
  const [graphBytes,sceneBytes]=await Promise.all([fs.readFile(graphPath),fs.readFile(scenePath)]);
  const graph=JSON.parse(graphBytes),scene=JSON.parse(sceneBytes);
  const changedPath=path.join(root,'target/benchmark/datasets',name+'.updated.incidence.json');
  let changed=null,changedSha=null,changedGraphSha=null,changedCounts=null;try{const bytes=await fs.readFile(changedPath);changed=JSON.parse(bytes);changedSha=sha(bytes);const updatedBytes=await fs.readFile(graphPath.replace(/\.json$/,'.updated.json'));changedGraphSha=sha(updatedBytes);changedCounts=assertions(JSON.parse(updatedBytes),changed);}catch(e){if(e.code!=='ENOENT')throw e;}
  report.datasets[name]={graphSha256:sha(graphBytes),incidenceSha256:sha(sceneBytes),...assertions(graph,scene),updatedIncidenceSha256:changedSha,updatedGraphSha256:changedGraphSha,updatedCounts:changedCounts,update:changed?'canonical shared updated incidence fixture':'Unavailable: canonical shared update fixture absent'};
  scenes.set(name,{scene,changed});
 }
 const order=[];for(let repetition=0;repetition<(validateOnly?1:repetitions);repetition++){
  const batch=[];for(const dataset of datasets)for(const tool of tools)for(const mode of (validateOnly?['static']:modes))batch.push({dataset,tool,mode,repetition});
  order.push(...shuffle(batch));
 }
 report.order=order;
 for(const job of order){
  const page=await context.newPage();let watchdog;let stage='load';
  try { const execute=async()=>{const errors=[];page.on('pageerror',e=>errors.push(String(e)));await page.goto(url);await page.waitForFunction(()=>window.benchmark);
  const {scene,changed}=scenes.get(job.dataset);const start=performance.now();
  stage='setup';const initialized=await page.evaluate(({tool,scene})=>window.benchmark.setup(tool,scene),{tool:job.tool,scene});const loadWallMs=performance.now()-start;
  stage='adapter validation';const topology=validateSnapshot(initialized.topology,scene);
  const graphics=await page.evaluate(()=>window.benchmark.graphics());
  const run={...job,status:'completed',framing:initialized.framing,validation:topology,settings:initialized.settings,kind:initialized.kind,graphics,errors};
  if(!validateOnly){stage='timed sampling';run.loadWallMs=loadWallMs;run.trace=await page.evaluate(config=>window.benchmark.sample(config),{mode:job.mode,warmup,samples});run.summary={javascriptCommand:distribution(run.trace.commandMs),rafWallInterval:distribution(run.trace.rafIntervalMs),averageThroughputFPS:run.trace.averageThroughputFPS};}
  if(changed){stage='incremental update';const updated=await page.evaluate(({scene,measure})=>window.benchmark.update(scene,measure),{scene:changed,measure:!validateOnly});
   run.incremental={...updated,topology:validateSnapshot(updated.topology,changed),unchangedNodeIDs:canonical(scene.nodes.map(n=>n.id).sort())===canonical(changed.nodes.map(n=>n.id).sort()),nodeCountDelta:changed.nodes.length-scene.nodes.length,linkCountDelta:changed.links.length-scene.links.length};}
  // Pixel readback outside timed sampling, once per tool/dataset only.
  stage='screenshot outside timing';if(job.repetition===0&&(job.mode==='static'||!modes.includes('static'))){const screenshot=path.join(output,`${job.dataset}-${job.tool}.png`);await page.screenshot({path:screenshot});run.screenshot=path.relative(root,screenshot);}
  if(errors.length)throw Error('Page errors '+errors.join('\n'));
  report.runs.push(run);
  await page.evaluate(()=>window.benchmark.dispose());
  console.log(`${validateOnly?'Validated':'Measured'} ${job.tool} ${job.dataset} ${job.mode} repetition ${job.repetition+1}`);
  };
  await Promise.race([execute(),new Promise((_,reject)=>{watchdog=setTimeout(()=>reject(Error('Job watchdog exceeded '+jobTimeoutMs+' ms')),jobTimeoutMs);})]);
  } catch(error){report.runs.push({...job,status:'failed',stage,error:String(error)});console.error(`Failed ${job.tool} ${job.dataset} ${stage}: ${error}`);process.exitCode=1;}
  finally{clearTimeout(watchdog);await page.close().catch(()=>{});await fs.writeFile(path.join(output,validateOnly?'validation.json':'results.json'),JSON.stringify(report,null,2));}
 }
 console.log('Evidence: '+path.join(output,validateOnly?'validation.json':'results.json'));
} finally{await browser.close();server.closeAllConnections();server.close();stop();}
