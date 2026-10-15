import {spawn} from 'node:child_process';
import {mkdirSync,writeFileSync,readFileSync,lstatSync,realpathSync,existsSync,renameSync} from 'node:fs';
import {tmpdir} from 'node:os';import {join,resolve,dirname} from 'node:path';import {randomBytes} from 'node:crypto';
import {pathToFileURL} from 'node:url';
import {SmokeError,verifyVault,verifyUi,verifyShutdown} from './installed-smoke-protocol.mjs';
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
const fail=code=>{throw new SmokeError(code);};
function readReceipt(path){
  const stat=lstatSync(path);if(!stat.isFile()||stat.isSymbolicLink()||stat.nlink!==1||stat.size>4096||realpathSync.native(path)!==path)fail('RECEIPT_FILE');
  return readFileSync(path,'utf8');
}
export function isolatedEnvironment(original,root,context){
  const env={};
  const osKeys=new Set(['path','pathext','systemroot','windir','comspec','temp','tmp','tmpdir','home','userprofile','localappdata','appdata','programfiles','programfiles(x86)','commonprogramfiles','commonprogramfiles(x86)','programdata','username','userdomain','user','logname','shell','lang','display','wayland_display','xauthority','xdg_runtime_dir','dbus_session_bus_address','number_of_processors','processor_architecture','__cf_user_text_encoding']);
  for(const [key,value] of Object.entries(original)){
    if(['temp','tmp','tmpdir'].includes(key.toLowerCase()))continue;
    if(!osKeys.has(key.toLowerCase())&&!/^LC_[A-Z_]+$/.test(key))continue;
    env[key]=value;
  }
  // Node and Rust/Windows prefer TEMP/TMP in different orders. Give this child
  // the same native parent selected by the controller, never a different root.
  return {...env,TEMP:dirname(root),TMP:dirname(root),TMPDIR:dirname(root),BIANK_DATA_ROOT:root,BIANK_DESKTOP_CONFIG_ROOT:join(root,'config'),BIANK_DESKTOP_DATA_ROOT:join(root,'data'),
    BIANK_INSTALLED_SMOKE_CONTEXT:context,BIANK_DESKTOP_DEV:'0',BIANK_STANDALONE:'1',BIANK_ACCOUNT_REQUIRED:'1',
    BIANK_ACCESS_ENV_FILE:join(root,'config','access.env'),BIANK_GOOGLE_CLIENT_SECRET_FILE:join(root,'config','google-client.secret')};
}
function track(child){
  const state={closed:false,code:null,signal:null,error:false};
  child.once('error',()=>{state.error=true;});
  child.once('close',(code,signal)=>{state.closed=true;state.code=code;state.signal=signal;});
  return state;
}
export async function validateSecondBoot(executable,candidateSha,ports={}){
  if(!/^[a-f0-9]{40}$/.test(candidateSha??''))fail('CANDIDATE_SHA');
  const launch=ports.spawn??spawn,delay=ports.sleep??sleep,now=ports.now??Date.now;
  const duration=ports.timeoutMs??150000;
  // Native realpath expands Windows 8.3 aliases as Rust canonicalize does.
  // The JS realpath implementation only resolves symlinks and retains RUNNER~1.
  const runId=randomBytes(16).toString('hex'),parent=realpathSync.native(tmpdir()),root=join(parent,'biank-installed-smoke-'+runId);
  mkdirSync(root,{mode:0o700});mkdirSync(join(root,'qa'),{mode:0o700});
  const contextPath=join(root,'qa','context.json'),initialNonce=randomBytes(32).toString('hex');
  const initial={schemaVersion:1,purpose:'cur160-installed-smoke',root,runId,initialNonce,candidateSha};
  writeFileSync(join(root,'qa','initial.json'),JSON.stringify(initial),{flag:'wx',mode:0o600});
  let first;const boots=[];
  for(const boot of [1,2]){
    const nonce=boot===1?initialNonce:randomBytes(32).toString('hex');
    const expected={root,runId,nonce,boot,candidateSha};
    const context={schemaVersion:1,purpose:initial.purpose,...expected};
    const staging=join(root,'qa','context.new');
    writeFileSync(staging,JSON.stringify(context),{flag:'wx',mode:0o600});renameSync(staging,contextPath);
    const started=now();let child;
    try{child=launch(resolve(executable),[],{env:isolatedEnvironment(ports.env??process.env,root,contextPath),stdio:'ignore'});}catch{fail('SHELL_START');}
    const state=track(child);let completed=false;
    try{
      const deadline=started+duration;
      while(now()<deadline){
        if(state.error)fail('SHELL_START');
        if(state.closed&&(state.code!==0||state.signal))fail('SHELL_EXIT');
        const vaultPath=join(root,'qa',`vault-boot-${boot}.json`),uiPath=join(root,'qa',`ui-boot-${boot}.json`),shutdownPath=join(root,'qa',`shutdown-boot-${boot}.json`);
        if(existsSync(vaultPath)&&existsSync(uiPath)&&existsSync(shutdownPath)){
          const vault=verifyVault(readReceipt(vaultPath),expected,first);
          verifyUi(readReceipt(uiPath),expected,vault,child.pid);
          verifyShutdown(readReceipt(shutdownPath),expected,vault,child.pid);
          if(state.closed){
            const gone=ports.engineGone??(pid=>{try{process.kill(pid,0);return false;}catch(error){return error.code==='ESRCH';}});
            if(!gone(vault.enginePid))fail('ENGINE_SURVIVED');
            if(boot===1)first=vault;
            boots.push({boot,shellPid:child.pid,enginePid:vault.enginePid,engineInstanceId:vault.engineInstanceId,nonce,durationMs:now()-started,vault:true,login:true,cleanShutdown:true});
            completed=true;break;
          }
        }
        if(state.closed)fail('RECEIPT_MISSING');
        await delay(100);
      }
      if(!completed)fail('BOOT_TIMEOUT');
    }finally{
      // A forced stop is cleanup only; it can never produce PASS or a second boot.
      if(!completed&&!state.closed){
        child.kill('SIGTERM');const deadline=now()+5000;
        while(!state.closed&&now()<deadline)await delay(50);
        if(!state.closed){
          child.kill('SIGKILL');const killedDeadline=now()+5000;
          while(!state.closed&&now()<killedDeadline)await delay(50);
          if(!state.closed)fail('CLEANUP_TIMEOUT');
        }
      }
    }
  }
  return {schemaVersion:1,status:'PASS',scope:'installed-identity-vault-and-local-login',candidateSha,runId,boots};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(resolve(process.argv[1])).href){
  try{
    const receipt=await validateSecondBoot(process.argv[2],process.argv[3]);
    if(process.argv[4])writeFileSync(resolve(process.argv[4]),JSON.stringify(receipt)+'\n',{flag:'wx',mode:0o600});
    console.log(JSON.stringify(receipt));
  }catch(error){console.error(JSON.stringify({status:'FAIL',code:error instanceof SmokeError?error.code:'HARNESS_FAILURE'}));process.exitCode=1;}
}
