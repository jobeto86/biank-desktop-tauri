import {test} from 'node:test';
import assert from 'node:assert/strict';
import {mkdtempSync,mkdirSync,writeFileSync,symlinkSync,lstatSync,readFileSync,rmSync} from 'node:fs';
import {join} from 'node:path';import {tmpdir} from 'node:os';
import {copyRuntime} from './runtime-links.mjs';
test('macOS materializes framework links before manifest hashing, other platforms retain them',()=>{
 const root=mkdtempSync(join(tmpdir(),'biank-framework-fixture-'));
 try{
  const source=join(root,'source');mkdirSync(join(source,'Versions/A'),{recursive:true});writeFileSync(join(source,'Versions/A/Framework'),'fixture');
  symlinkSync('A',join(source,'Versions/Current'),'junction');symlinkSync('Versions/Current/Framework',join(source,'Framework'));
  const mac=join(root,'mac');copyRuntime(source,mac,'darwin');assert.equal(lstatSync(join(mac,'Framework')).isSymbolicLink(),false);assert.equal(lstatSync(join(mac,'Versions/Current')).isSymbolicLink(),false);assert.equal(readFileSync(join(mac,'Framework'),'utf8'),'fixture');
  if(process.platform!=='win32'){const linux=join(root,'linux');copyRuntime(source,linux,'linux');assert.equal(lstatSync(join(linux,'Framework')).isSymbolicLink(),true);}
  symlinkSync(join(root,'outside'),join(source,'external'));writeFileSync(join(root,'outside'),'outside');assert.throws(()=>copyRuntime(source,join(root,'invalid'),'darwin'),/fuera del runtime/);
 }finally{rmSync(root,{recursive:true,force:true});}
});
