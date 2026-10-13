import {spawn} from 'node:child_process';
import {readFile, writeFile, mkdir, mkdtemp, cp, rm} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {dirname, resolve, join} from 'node:path';
import {build} from 'esbuild';
import {packageBackend, publish} from './artifacts.mjs';
import {generateNotices} from './notices.mjs';
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const web=join(root,'web');
const profile=process.env.HYPER_WEB_PROFILE ?? 'web-release';
const bindgen=process.env.WASM_BINDGEN ?? 'wasm-bindgen';
const metadata=JSON.parse(await readFile(join(web,'package.json'),'utf8'));
function run(command,args) {
  return new Promise((resolve,reject)=>{
    const child=spawn(command,args,{cwd:root,stdio:'inherit'});
    child.on('error',reject);child.on('exit',code=>code===0?resolve():reject(Error(`${command} exited ${code}`)));
  });
}
const lock=await readFile(join(root,'Cargo.lock'),'utf8');
const version=lock.match(/name = "wasm-bindgen"\nversion = "([^"]+)"/)[1];
try { await run(bindgen,['--version']); }
catch { throw Error(`Install wasm-bindgen-cli ${version} or set WASM_BINDGEN to its executable`); }
const staging=await mkdtemp(join(web,'.build-'));
try {
  const manifest={schema:1,version:metadata.version};
  for(const backend of ['webgpu','webgl2']) {
    await run('cargo',['build','--locked','-p','hyper-viz-web','--target','wasm32-unknown-unknown',
      '--profile',profile,'--no-default-features','--features',backend]);
    const generated=join(staging,'generated',backend);await mkdir(generated,{recursive:true});
    await run(bindgen,[join(root,'target','wasm32-unknown-unknown',profile,'hyper_viz_web.wasm'),
      '--target','web','--out-dir',generated,'--out-name','hyper']);
    manifest[backend]=await packageBackend(generated,staging,backend);
  }
  await build({entryPoints:[join(web,'src/index.mjs')],bundle:true,format:'esm',target:'es2022',outfile:join(staging,'index.mjs')});
  await cp(join(web,'src/index.d.ts'),join(staging,'index.d.ts'));
  await writeFile(join(staging,'renderers.json'),JSON.stringify(manifest,null,2)+'\n');
  await cp(join(web,'demo'),join(staging,'demo'),{recursive:true});
  await cp(join(root,'fixtures/sample.json'),join(staging,'demo/sample.json'));
  await cp(join(root,'fixtures/notebook-small.hif.json'),join(staging,'demo/sample.hif.json'));
  await rm(join(staging,'generated'),{recursive:true});
  await generateNotices(root,staging);
  // Both backends must succeed before replacing the previous output.
  const dist=join(web,'dist');
  await publish(staging,dist);
  for (const name of ['LICENSE','LICENSE-MIT','LICENSE-APACHE','NOTICE']) await cp(join(root,name),join(web,name));
  console.log(`Browser SDK built at ${dist}`);
} finally {await rm(staging,{recursive:true,force:true});}
