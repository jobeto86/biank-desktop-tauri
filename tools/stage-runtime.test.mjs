import test from 'node:test';
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {mkdtempSync,mkdirSync,writeFileSync,readFileSync,readdirSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {dirname,join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {resolveCoreSha,stageRuntime} from './stage-runtime.mjs';

const repo=join(dirname(fileURLToPath(import.meta.url)),'..');
// Read the real checkout, never substitute an expected/env SHA for provenance.
const actualSha=execFileSync('git',['rev-parse','--verify','HEAD'],{cwd:repo,encoding:'utf8',shell:false}).trim();
const otherSha=actualSha==='a'.repeat(40)?'b'.repeat(40):'a'.repeat(40);
function gitFixture({sha=actualSha,status=''}={}){
 const calls=[];
 const runGit=(file,args,options)=>{
  calls.push({file,args,options});
  if(args[0]==='rev-parse')return sha+'\n';
  if(args[0]==='status')return status;
  throw Error('Unexpected git invocation');
 };
 return {calls,runGit};
}

test('provenance uses actual HEAD and tracked/untracked status without shell, including hostile cwd data',()=>{
 const f=gitFixture(),desktop=join(tmpdir(),'core with spaces; echo forged');
 assert.equal(resolveCoreSha(desktop,{expectedSha:actualSha,runGit:f.runGit}),actualSha);
 assert.deepEqual(f.calls.map(({file,args})=>({file,args})),[
  {file:'git',args:['rev-parse','--verify','HEAD']},
  {file:'git',args:['status','--porcelain','--untracked-files=normal']},
 ]);
 for(const {options} of f.calls){
  assert.equal(options.cwd,desktop);assert.equal(options.shell,false);
  assert.deepEqual(options.stdio,['ignore','pipe','pipe']);assert.equal(options.timeout,10000);
 }
});

test('expected SHA is optional but cannot forge, abbreviate or replace actual HEAD',()=>{
 assert.equal(resolveCoreSha(repo,{runGit:gitFixture().runGit}),actualSha);
 assert.throws(()=>resolveCoreSha(repo,{expectedSha:otherSha,runGit:gitFixture().runGit}),/no coincide/);
 for(const expectedSha of ['',actualSha.slice(0,7),'A'.repeat(40),'x'.repeat(40),actualSha+'\n']){
  let calls=0;
  assert.throws(()=>resolveCoreSha(repo,{expectedSha,runGit:()=>{calls++;}}),/40 caracteres/);
  assert.equal(calls,0);
 }
 for(const sha of ['',actualSha.slice(0,7),'x'.repeat(40)]){
  assert.throws(()=>resolveCoreSha(repo,{runGit:gitFixture({sha}).runGit}),/HEAD del core/);
 }
});

test('tracked and untracked changes reject provenance without exposing Git paths or output',()=>{
 for(const status of [' M src/index.ts\n','M  src/index.ts\n',' D src/index.ts\n','UU src/index.ts\n',' m module\n','?? src/untracked.ts\n','?? new-source/\n']){
  assert.throws(()=>resolveCoreSha(repo,{runGit:gitFixture({status}).runGit}),{
   message:'El core tiene cambios tracked/untracked; no se puede atribuir el runtime a HEAD.',
  });
 }
 for(const failedCommand of ['rev-parse','status']){
  const f=gitFixture();
  assert.throws(()=>resolveCoreSha(repo,{expectedSha:actualSha,runGit:(file,args,options)=>{
   if(args[0]===failedCommand)throw Error('private git diagnostics');
   return f.runGit(file,args,options);
  }}),/^Error: No se pudo verificar HEAD y el estado tracked\/untracked del core\.$/);
 }
});

test('real git permits ignored build/dependency artifacts but rejects untracked source and tracked modifications',t=>{
 const root=mkdtempSync(join(tmpdir(),'biank-core-git-'));t.after(()=>rmSync(root,{recursive:true,force:true}));
 const git=args=>execFileSync('git',args,{cwd:root,encoding:'utf8',shell:false,stdio:['ignore','pipe','pipe']});
 git(['init','--quiet']);
 writeFileSync(join(root,'.gitignore'),'dist/\nnode_modules/\n');writeFileSync(join(root,'source'),'committed');
 git(['add','--','.gitignore','source']);
 // Fixture-only commit in a new temporary repository; no public checkout mutation.
 git(['-c','user.name=Fixture','-c','user.email=fixture@example.invalid','-c','commit.gpgsign=false','commit','--quiet','-m','fixture']);
 const head=git(['rev-parse','HEAD']).trim();
 mkdirSync(join(root,'dist'));writeFileSync(join(root,'dist','ignored'),'build');
 mkdirSync(join(root,'node_modules'));writeFileSync(join(root,'node_modules','ignored'),'dependency');
 assert.equal(resolveCoreSha(root),head);
 mkdirSync(join(root,'src'));writeFileSync(join(root,'src','untracked.mjs'),'export const source="untracked";');
 assert.throws(()=>resolveCoreSha(root),{message:'El core tiene cambios tracked/untracked; no se puede atribuir el runtime a HEAD.'});
 rmSync(join(root,'src'),{recursive:true});assert.equal(resolveCoreSha(root),head);
 writeFileSync(join(root,'source'),'changed');assert.throws(()=>resolveCoreSha(root),/cambios tracked/);
});

test('staged config binds real HEAD bytes into the resource manifest and retains googleIdentity',t=>{
 const root=mkdtempSync(join(tmpdir(),'biank-stage-binding-'));t.after(()=>rmSync(root,{recursive:true,force:true}));
 const desktop=join(root,'core','apps','desktop');
 mkdirSync(join(root,'src-tauri'));mkdirSync(join(desktop,'dist','coordinator-runtime'),{recursive:true});
 mkdirSync(join(desktop,'dist','web-vite'),{recursive:true});
 writeFileSync(join(desktop,'dist','coordinator-runtime','index.mjs'),'fixture runtime');
 writeFileSync(join(desktop,'dist','web-vite','index.html'),'fixture web');
 const env={BIANK_STAGE_DEVELOPMENT:'1',BIANK_CORE_SHA:actualSha,BIANK_GOOGLE_CLIENT_ID:'fixture-client'};
 const f=gitFixture();
 const result=stageRuntime({root,desktop,env,runGit:f.runGit});
 assert.equal(result.coreSha,actualSha);assert.equal(f.calls.length,4);
 const bytes=readFileSync(join(result.runtime,'runtime-config.json'));
 assert.deepEqual(JSON.parse(bytes),{coreSha:actualSha,googleIdentity:{clientId:'fixture-client'}});
 const manifest=JSON.parse(readFileSync(join(result.runtime,'resources-manifest.json'),'utf8'));
 const boundHash=manifest.files['runtime-config.json'].sha256;
 assert.equal(boundHash,createHash('sha256').update(bytes).digest('hex'));
 const forged=Buffer.from(bytes.toString().replace(actualSha,otherSha));
 assert.notEqual(boundHash,createHash('sha256').update(forged).digest('hex'));
 // Development skips missing optional binaries, never Git checks or expected SHA validation.
 assert.throws(()=>stageRuntime({root,desktop,env:{...env,BIANK_CORE_SHA:otherSha},runGit:f.runGit}),/no coincide/);
 assert.throws(()=>stageRuntime({root,desktop,env,runGit:gitFixture({status:' M source\n'}).runGit}),/cambios tracked/);
 assert.throws(()=>stageRuntime({root,desktop,env,runGit:gitFixture({status:'?? src/untracked.ts\n'}).runGit}),{
  message:'El core tiene cambios tracked/untracked; no se puede atribuir el runtime a HEAD.',
 });
 assert.throws(()=>stageRuntime({root,desktop,env,runGit:()=>{throw Error('no git');}}),/No se pudo verificar/);
 const before=readFileSync(join(result.runtime,'resources-manifest.json'),'utf8');
 let reads=0;
 assert.throws(()=>stageRuntime({root,desktop,env,runGit:(file,args)=>args[0]==='status'?'':(++reads===1?actualSha:otherSha)+'\n'}),/no coincide/);
 assert.equal(readFileSync(join(result.runtime,'resources-manifest.json'),'utf8'),before);
 assert.equal(readdirSync(join(root,'src-tauri')).includes('runtime'),true);
});
