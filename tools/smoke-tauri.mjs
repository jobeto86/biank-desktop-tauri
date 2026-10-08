import {mkdtempSync,readFileSync,existsSync,writeFileSync,openSync,closeSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join,resolve,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawn,execFileSync} from 'node:child_process';
import assert from 'node:assert/strict';
import {randomBytes} from 'node:crypto';
const repository=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const root=mkdtempSync(join(tmpdir(),'biank-tauri-smoke-'));
const log=openSync(join(root,'shell.log'),'a');
const child=spawn(join(repository,'src-tauri/target/debug/biank-desktop'),[],{cwd:repository,env:{...process.env,BIANK_TAURI_TEST_ROOT:root,WEBKIT_SKIA_ENABLE_CPU_RENDERING:'1',WEBKIT_DISABLE_DMABUF_RENDERER:'1',WEBKIT_DISABLE_COMPOSITING_MODE:'1',LIBGL_ALWAYS_SOFTWARE:'1',VK_DRIVER_FILES:'/usr/share/vulkan/icd.d/lvp_icd.json'},stdio:['ignore',log,log]});
closeSync(log);
let evidence=null,gracefulExit=false;
try{
 const until=Date.now()+120000;
 while(Date.now()<until){
  if(child.exitCode!==null)throw Error(`Tauri terminó con código ${child.exitCode}; evidencia en ${root}`);
  if(existsSync(join(root,'qa-ui.json'))){try{evidence=JSON.parse(readFileSync(join(root,'qa-ui.json'),'utf8'));}catch{}if(evidence?.authBrowser&&evidence?.bridge&&evidence?.health&&evidence?.login)break;}
  await new Promise(resolve=>setTimeout(resolve,500));
 }
 assert.deepEqual(evidence,{authBrowser:true,bridge:true,health:true,login:true},`La ventana no acredita UI, IPC y motor; evidencia en ${root}`);
 const ids=execFileSync('xdotool',['search','--name','^Biank Desktop$'],{encoding:'utf8'}).trim().split('\n');
 execFileSync('import',['-window',ids.at(-1),join(root,'tauri-login.png')]);
 execFileSync('xdotool',['windowfocus','--sync',ids.at(-1)]);
 execFileSync('xdotool',['key','--clearmodifiers','ctrl+shift+q']);
 const exitDeadline=Date.now()+20000;
 while(child.exitCode===null&&Date.now()<exitDeadline)await new Promise(resolve=>setTimeout(resolve,100));
 assert.equal(child.exitCode,0,'El cierre nativo no terminó de forma ordenada');
 gracefulExit=true;
 writeFileSync(join(root,'result.json'),JSON.stringify({status:'PASS',...evidence,gracefulExit:true})+'\n');
 console.log(JSON.stringify({status:'PASS',nativeBridge:true,authenticatedHealth:true,loginSurface:true,gracefulExit:true,evidence:root}));
}finally{
 // Shutdown only the engine belonging to this synthetic installation, then its QA shell.
 if(!gracefulExit&&existsSync(join(root,'runtime.json'))){
  const runtime=JSON.parse(readFileSync(join(root,'runtime.json'),'utf8'));
  const token=readFileSync(join(root,'secrets/web.token'),'utf8').trim();
  const headers={'X-Biank-Instance':runtime.instanceId,'X-Biank-Local-Token':token,'Content-Type':'application/json'};
  const origin=`http://127.0.0.1:${runtime.port}`;
  try{
   const health=await fetch(origin+'/api/health',{headers}).then(r=>r.json());
   assert.equal(health.nativeInstanceId,runtime.instanceId);
   const owner=randomBytes(32).toString('hex');
   const state=await fetch(origin+'/api/desktop/drain',{method:'POST',headers,body:JSON.stringify({enabled:true,owner})}).then(r=>r.json());
   assert.equal(state.active,0);
   await fetch(origin+'/api/desktop/shutdown',{method:'POST',headers,body:JSON.stringify({owner})});
  }catch(error){console.error(`Limpieza del motor QA pendiente en ${root}: ${error.message}`);}
 }
 if(child.exitCode===null){child.kill('SIGTERM');await new Promise(resolve=>child.once('exit',resolve));}
}
