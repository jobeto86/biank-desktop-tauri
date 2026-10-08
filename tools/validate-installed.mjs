import {spawn,execFileSync} from 'node:child_process';
import {mkdtempSync,existsSync,readFileSync,openSync,closeSync,writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join,resolve} from 'node:path';
import assert from 'node:assert/strict';
import {randomBytes} from 'node:crypto';
const executable=resolve(process.argv[2]);
const root=mkdtempSync(join(tmpdir(),'biank-installed-'));
const log=openSync(join(root,'shell.log'),'a');
const child=spawn(executable,[],{env:{...process.env,BIANK_DATA_ROOT:root,BIANK_DESKTOP_CONFIG_ROOT:join(root,'config')},stdio:['ignore',log,log]});closeSync(log);
let connection;
try{
 const deadline=Date.now()+150000;
 while(Date.now()<deadline){
  if(child.exitCode!==null)throw Error(`Installed shell exited ${child.exitCode}`);
  try{
   const runtime=JSON.parse(readFileSync(join(root,'runtime.json'),'utf8'));
   const token=readFileSync(join(root,'secrets/web.token'),'utf8').trim();
   const origin=`http://127.0.0.1:${runtime.port}`,headers={'X-Biank-Instance':runtime.instanceId,'X-Biank-Local-Token':token,'Content-Type':'application/json'};
   const health=await fetch(origin+'/api/health',{headers,signal:AbortSignal.timeout(2000)}).then(r=>r.json());
   if(health.service==='biank-desktop'&&health.nativeInstanceId===runtime.instanceId){connection={origin,headers};break;}
  }catch{}
  await new Promise(r=>setTimeout(r,500));
 }
 assert.ok(connection,`Installed engine failed; synthetic evidence ${root}`);
 const ui=await fetch(connection.origin+'/',{headers:connection.headers}).then(r=>r.text());
 assert.match(ui,/<!doctype html/i);
 const deadlineWindow=Date.now()+30000;
 while(Date.now()<deadlineWindow&&!readFileSync(join(root,'shell.log'),'utf8').includes('biank-shell: ventana principal lista'))await new Promise(r=>setTimeout(r,250));
 assert.match(readFileSync(join(root,'shell.log'),'utf8'),/biank-shell: ventana principal lista/,'Native webview did not initialize');
 const runtime=join(process.platform==='darwin'?resolve(executable,'../../Resources'):resolve(executable,'..'),'runtime');
 execFileSync(join(runtime,'codex',process.platform==='win32'?'codex.exe':'codex'),['--version'],{stdio:'pipe'});
 console.log(JSON.stringify({status:'PASS',installedEngine:true,nativeWindow:true,bundledCodex:true,evidence:root}));
 writeFileSync(join(root,'result.json'),JSON.stringify({status:'PASS',installedEngine:true,nativeWindow:true,bundledCodex:true})+'\n');
}finally{
 if(connection){const owner=randomBytes(32).toString('hex');
  const request=(route,body)=>fetch(connection.origin+route,{method:'POST',headers:connection.headers,body:JSON.stringify(body)});
  const state=await request('/api/desktop/drain',{enabled:true,owner}).then(r=>r.json());assert.equal(state.active,0);
  await request('/api/desktop/shutdown',{owner});
 }
 if(child.exitCode===null)child.kill();
}
