import {readFile, writeFile, mkdir, rename, rm} from 'node:fs/promises';
import {join} from 'node:path';
import {createHash} from 'node:crypto';
import {bindingFactory} from './factory.mjs';

export async function packageBackend(generated, destination, backend) {
  const wasm=await readFile(join(generated,'hyper_bg.wasm'));
  const source=await bindingFactory(join(generated,'hyper.js'));
  const digest=createHash('sha256').update(wasm).update(source).digest('hex').slice(0,16);
  const directory=join(destination,'renderers',backend);
  await mkdir(directory,{recursive:true});
  const moduleName=`hyper-${digest}.mjs`, wasmName=`hyper-${digest}.wasm`;
  await writeFile(join(directory,moduleName),source);
  await writeFile(join(directory,wasmName),wasm);
  return {module:`renderers/${backend}/${moduleName}`,wasm:`renderers/${backend}/${wasmName}`};
}

/** Preserve the old output if installing a fully assembled build fails. */
export async function publish(staging, destination) {
  const previous=`${destination}.previous`;
  await rm(previous,{recursive:true,force:true});
  try {await rename(destination,previous);} catch(error) {if(error.code!=='ENOENT')throw error;}
  try {await rename(staging,destination);}
  catch(error) {await rename(previous,destination).catch(()=>{});throw error;}
  await rm(previous,{recursive:true,force:true});
}
