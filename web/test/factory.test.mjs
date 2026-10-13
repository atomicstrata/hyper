import {test} from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp, writeFile, rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {bindingFactory} from '../factory.mjs';

test('one imported factory creates independent binding state without eval',async()=>{
  const dir=await mkdtemp(join(tmpdir(),'hyper-factory-'));
  try {
    const entry=join(dir,'bindings.js');
    await writeFile(entry,'let value = 0; export function next() { return ++value; }');
    const source=await bindingFactory(entry);
    const {createBindings}=await import(`data:text/javascript,${encodeURIComponent(source)}`);
    const a=createBindings(), b=createBindings();
    assert.equal(a.next(),1); assert.equal(a.next(),2); assert.equal(b.next(),1);
  } finally {await rm(dir,{recursive:true,force:true});}
});
