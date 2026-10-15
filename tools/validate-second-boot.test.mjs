import test from 'node:test';import assert from 'node:assert/strict';import {EventEmitter} from 'node:events';
import {readFileSync,writeFileSync,readdirSync,rmSync} from 'node:fs';import {join} from 'node:path';
import {rootDigest,SmokeError} from './installed-smoke-protocol.mjs';
import {validateSecondBoot,isolatedEnvironment} from './validate-second-boot.mjs';
const sha='c'.repeat(40);
function harness(t,mode='good'){
 const calls=[],roots=new Set(),kills=[];let clock=0;
 t.after(()=>{for(const root of roots)rmSync(root,{recursive:true,force:true});});
 const spawn=(_executable,args,options)=>{
  assert.deepEqual(args,[]);assert.equal(options.stdio,'ignore');
  const context=JSON.parse(readFileSync(options.env.BIANK_INSTALLED_SMOKE_CONTEXT,'utf8')),root=context.root;
  roots.add(root);calls.push(context);
  if(context.boot===1){assert.deepEqual(readdirSync(root),['qa']);assert.deepEqual(readdirSync(join(root,'qa')).sort(),['context.json','initial.json']);}
  const child=new EventEmitter();child.pid=100+context.boot;
  child.kill=signal=>{kills.push(signal);queueMicrotask(()=>child.emit('close',null,signal));return true;};
  if(mode==='timeout')return child;
  const common={schemaVersion:1,runId:context.runId,nonce:context.nonce,boot:context.boot,candidateSha:context.candidateSha,rootSha256:rootDigest(root),engineInstanceId:'engine-0000000000-'+context.boot,enginePid:200+context.boot};
  if(mode==='reuse-pid'&&context.boot===2)common.enginePid=201;
  const vault={...common,kind:'vault',operation:context.boot===1?'put':'resolve',digestSha256:'d'.repeat(64)};
  if(mode==='stale'&&context.boot===2)vault.nonce=calls[0].nonce;
  const ui={...common,kind:'ui',shellPid:child.pid,health:true,login:mode!=='bad-ui',accountRequired:true,unauthenticated:true,bridge:true};
  const shutdown={...common,kind:'shutdown',shellPid:child.pid,drained:mode!=='bad-drain',engineExited:true};
  if(mode!=='missing')for(const [kind,value] of [['vault',vault],['ui',ui],['shutdown',shutdown]])writeFileSync(join(root,'qa',`${kind}-boot-${context.boot}.json`),JSON.stringify(value),{flag:'wx'});
  queueMicrotask(()=>child.emit('close',mode==='exit-failure'?1:0,null));
  return child;
 };
 return {calls,kills,ports:{spawn,env:{HOME:'/os-home'},sleep:async()=>{},now:()=>clock+=100,timeoutMs:1000,engineGone:()=>mode!=='engine-survived'}};
}
test('controller waits for clean shutdown and uses the same root with new processes/nonces',async t=>{
 const h=harness(t),receipt=await validateSecondBoot('/candidate',sha,h.ports);
 assert.equal(receipt.status,'PASS');assert.equal(h.calls.length,2);assert.equal(h.kills.length,0);
 assert.equal(h.calls[0].root,h.calls[1].root);assert.notEqual(h.calls[0].nonce,h.calls[1].nonce);
 assert.equal(receipt.boots.length,2);assert.notEqual(receipt.boots[0].enginePid,receipt.boots[1].enginePid);
 assert.equal(JSON.stringify(receipt).includes(h.calls[0].root),false,'receipt must not retain the synthetic profile path');
});
for(const [mode,code,launches] of [['stale','RECEIPT_INVALID',2],['reuse-pid','RECEIPT_INVALID',2],['missing','RECEIPT_MISSING',1],['bad-ui','RECEIPT_INVALID',1],['bad-drain','RECEIPT_INVALID',1],['exit-failure','SHELL_EXIT',1],['engine-survived','ENGINE_SURVIVED',1],['timeout','BOOT_TIMEOUT',1]])test(`controller fails closed: ${mode}`,async t=>{
 const h=harness(t,mode);await assert.rejects(validateSecondBoot('/candidate',sha,h.ports),error=>error instanceof SmokeError&&error.code===code);
 assert.equal(h.calls.length,launches);if(mode==='timeout')assert.deepEqual(h.kills,['SIGTERM']);
});
test('launch environment preserves native OS paths and drops unrelated variable references',()=>{
 const excluded=['ACTIONS_RUNTIME_TOKEN','GITHUB_TOKEN','BIANK_DESKTOP_VAULT_KEY_FILE','BIANK_TAURI_TEST_ROOT','NODE_OPTIONS','PRIVATE_UNRECOGNIZED_TOKEN'];
 // Presence alone must be removed, including undefined values; no credential
 // material is needed to check that the environment uses an allowlist.
 const original={Path:'native-path',SystemRoot:'native-root',HOME:'native-home',DBUS_SESSION_BUS_ADDRESS:'session-address',LANG:'es_MX.UTF-8',LC_ALL:'C',...Object.fromEntries(excluded.map(name=>[name,undefined]))};
 const env=isolatedEnvironment(original,'/tmp/synthetic','/tmp/synthetic/qa/context.json');
 assert.equal(env.Path,'native-path');assert.equal(env.SystemRoot,'native-root');assert.equal(env.HOME,'native-home');assert.equal(env.DBUS_SESSION_BUS_ADDRESS,'session-address');assert.equal(env.LC_ALL,'C');
 for(const name of excluded)assert.equal(Object.hasOwn(env,name),false);
 assert.equal(env.BIANK_DESKTOP_DEV,'0');assert.equal(env.BIANK_STANDALONE,'1');
 assert.equal(env.BIANK_ACCESS_ENV_FILE,join('/tmp/synthetic','config','access.env'));
});
test('a mutable or absent candidate identifier cannot start a process',async()=>{
 for(const value of [undefined,'main','abc','C'.repeat(40)])await assert.rejects(validateSecondBoot('/candidate',value,{spawn:()=>assert.fail('must not launch')}),error=>error.code==='CANDIDATE_SHA');
});
test('forced cleanup waits for close after SIGKILL and never starts boot2',async t=>{
 const h=harness(t,'timeout');let child,killed=false,polls=0,closed=false;
 const launch=h.ports.spawn;
 h.ports.spawn=(...args)=>{
  child=launch(...args);
  child.kill=signal=>{h.kills.push(signal);if(signal==='SIGKILL')killed=true;return true;};
  return child;
 };
 h.ports.sleep=async()=>{if(killed&&++polls===3){closed=true;child.emit('close',null,'SIGKILL');}};
 await assert.rejects(validateSecondBoot('/candidate',sha,h.ports),error=>error.code==='BOOT_TIMEOUT');
 assert.equal(closed,true);assert.equal(polls,3);assert.deepEqual(h.kills,['SIGTERM','SIGKILL']);assert.equal(h.calls.length,1);
});
