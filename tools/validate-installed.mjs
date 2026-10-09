import {spawn,execFileSync} from 'node:child_process';
import {mkdtempSync,existsSync,readFileSync,openSync,closeSync,writeFileSync,mkdirSync,readdirSync,statfsSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join,resolve} from 'node:path';
import assert from 'node:assert/strict';
import {randomBytes} from 'node:crypto';
const executable=resolve(process.argv[2]);
const root=mkdtempSync(join(tmpdir(),'biank-installed-'));
// Upgrade QA: a profile left by an earlier version forces the pre-migration backup.
// Large enough that the backup outlasts the retired fixed 120 s startup deadline.
const upgradeMb=Number(process.env.BIANK_QA_UPGRADE_PROFILE_MB||0);
if(upgradeMb>0){
 writeFileSync(join(root,'desktop-version.json'),JSON.stringify({version:process.env.BIANK_QA_UPGRADE_FROM||'0.3.5',backup:null}));
 const chunk=randomBytes(256*1024);
 for(let i=0;i<upgradeMb*4;i++){
  const dir=join(root,'workspace','qa-upgrade-profile',String(Math.floor(i/500)));
  if(i%500===0)mkdirSync(dir,{recursive:true});
  writeFileSync(join(dir,i+'.bin'),Buffer.concat([chunk,Buffer.from(String(i))]));
 }
}
const startedAt=Date.now();
const log=openSync(join(root,'shell.log'),'a');
const child=spawn(executable,[],{env:{...process.env,BIANK_DATA_ROOT:root,BIANK_DESKTOP_CONFIG_ROOT:join(root,'config')},stdio:['ignore',log,log]});closeSync(log);
let connection;
try{
 const deadline=Date.now()+(upgradeMb>0?660000:150000);
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
 const ui=await fetch(connection.origin+'/',{headers:connection.headers,signal:AbortSignal.timeout(10000)}).then(r=>r.text());
 assert.match(ui,/<!doctype html/i);
 const deadlineWindow=Date.now()+30000;
 while(Date.now()<deadlineWindow&&(!existsSync(join(root,'logs/shell.log'))||!readFileSync(join(root,'logs/shell.log'),'utf8').includes('biank-shell: ventana principal lista')))await new Promise(r=>setTimeout(r,250));
 assert.match(readFileSync(join(root,'logs/shell.log'),'utf8'),/biank-shell: ventana principal lista/,'Native webview did not initialize');
 const runtime=process.argv[3]?resolve(process.argv[3]):join(process.platform==='darwin'?resolve(executable,'../../Resources'):resolve(executable,'..'),'runtime');
 execFileSync(join(runtime,'codex',process.platform==='win32'?'codex.exe':'codex'),['--version'],{stdio:'pipe'});
 let upgrade;
 if(upgradeMb>0){
  const marker=JSON.parse(readFileSync(join(root,'desktop-version.json'),'utf8'));
  assert.notEqual(marker.version,process.env.BIANK_QA_UPGRADE_FROM||'0.3.5','Version marker was not advanced after the upgrade backup');
  assert.ok(marker.backup&&existsSync(marker.backup),'Pre-migration backup was not published');
  assert.deepEqual(readdirSync(join(root,'backups')).filter(name=>name.startsWith('.snapshot-')),[],'An unpublished backup staging remained');
  upgrade={profileMb:upgradeMb,startupSeconds:Math.round((Date.now()-startedAt)/1000),version:marker.version};
 }
 console.log(JSON.stringify({status:'PASS',installedEngine:true,nativeWindow:true,bundledCodex:true,...(upgrade?{upgrade}:{}),evidence:root}));
 writeFileSync(join(root,'result.json'),JSON.stringify({status:'PASS',installedEngine:true,nativeWindow:true,bundledCodex:true})+'\n');
}catch(error){
 // Only the Rust shell's controlled bootstrap messages; no engine tokens/logs.
 if(existsSync(join(root,'logs/shell.log')))console.error(readFileSync(join(root,'logs/shell.log'),'utf8'));
 if(existsSync(join(root,'logs/engine.log'))){
  const text=readFileSync(join(root,'logs/engine.log'),'utf8');
  const codes=[...new Set(text.match(/\b(?:ERR_[A-Z_]+|MODULE_NOT_FOUND|EACCES|ENOENT|SQLITE_[A-Z_]+)\b/g)||[])];
  const modules=[...text.matchAll(/Cannot find (?:module|package) ['"]([@a-zA-Z0-9_./-]+)['"]/g)].map(m=>m[1]);
  const classes=[...new Set(text.match(/\b(?:Error|TypeError|ReferenceError|SyntaxError|VaultError|VaultLocked)\b/g)||[])];
  const symptoms={stdin:/stdin/i.test(text),vault:/vault|llave|bóveda/i.test(text),catalog:/catálogo interno|base.skills|semilla/i.test(text),path:/URL|path|ruta|directorio/i.test(text)};
  console.error(JSON.stringify({nodeStartupErrorCodes:codes,errorClasses:classes,missingPackages:modules,symptoms}));
 }
 // Synthetic upgrade roots hold no user data or secrets: surface the engine's own error lines.
 if(upgradeMb>0&&existsSync(join(root,'logs/engine.log'))){
  const lines=readFileSync(join(root,'logs/engine.log'),'utf8').split(/\r?\n/).filter(line=>/error|Error|ENOSPC|EPERM|EBUSY|respaldo|integridad/.test(line)).slice(-15);
  console.error(JSON.stringify({upgradeEngineErrors:lines.map(line=>line.replace(/[A-Za-z0-9_-]{40,}/g,'<redacted>').slice(0,400))}));
 }
 if(upgradeMb>0)try{console.error(JSON.stringify({freeDiskBytes:statfsSync(root).bavail*statfsSync(root).bsize}));}catch{}
 if(existsSync(join(root,'runtime-phase.json'))){const phase=JSON.parse(readFileSync(join(root,'runtime-phase.json'),'utf8')).phase;if(/^[a-z][a-z0-9_-]{0,40}$/.test(phase||''))console.error(JSON.stringify({lastBootstrapPhase:phase}));}
 throw error;
}finally{
 if(connection){const owner=randomBytes(32).toString('hex');
  const request=(route,body)=>fetch(connection.origin+route,{method:'POST',headers:connection.headers,body:JSON.stringify(body)});
  const state=await request('/api/desktop/drain',{enabled:true,owner}).then(r=>r.json());assert.equal(state.active,0);
  await request('/api/desktop/shutdown',{owner});
 }
 if(child.exitCode===null)child.kill();
}
