import test from 'node:test';import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';import vm from 'node:vm';
const source=readFileSync(new URL('./installed-smoke-bridge.js',import.meta.url),'utf8');
async function probe(change={}){
 const calls=[],listeners=new Map();let clock=-61000,finish;
 const done=new Promise(resolve=>{finish=resolve;});
 const button={disabled:false,getBoundingClientRect:()=>({width:100,height:40,left:10,top:10}),contains:()=>false,focus:()=>{document.activeElement=button;}};
 const dialog={open:true,matches:()=>true,querySelector:()=>button};button.parentElement=dialog;
 const document={readyState:'complete',activeElement:null,querySelector:()=>dialog,elementFromPoint:()=>button};
 const health={service:'biank-desktop',nativeInstanceId:'native-engine-00001'},account={required:true,authenticated:false,user:null};
 const style={display:'block',visibility:'visible',opacity:'1'};
 const options={button,dialog,document,health,account,style};change.configure?.(options);
 const context={nonce:'b'.repeat(64),engineInstanceId:'native-engine-00001',origin:'http://127.0.0.1:1234'};
 const sandbox={location:{origin:change.origin??context.origin},document,innerWidth:1280,innerHeight:800,AbortSignal,
  Date:{now:()=>clock+=61000},getComputedStyle:element=>element.styleOverride??style,setTimeout:fn=>queueMicrotask(fn),
  addEventListener:(name,fn)=>listeners.set(name,fn),
  fetch:async(path,init)=>{calls.push({path,init});return {ok:true,json:async()=>path==='/api/health'?health:account};},
  window:{biankDesktop:{app:{quit:()=>{throw new Error('Must not quit through the normal updater path');}}},
   __TAURI__:{core:{invoke:async(command,args)=>{calls.push({command,args});if(command==='get_desktop_info')return {edition:'Tauri v2'};finish(args.report);}}}},
 };
 vm.runInNewContext(source.replace('__CUR160_SMOKE_CONTEXT__',JSON.stringify(context)),sandbox);
 if(change.origin&&change.origin!==context.origin){await new Promise(resolve=>setImmediate(resolve));return {calls};}
 if(change.loading){assert.equal(calls.length,0);listeners.get('DOMContentLoaded')();}
 const report=await Promise.race([done,new Promise((_,reject)=>{const timer=setTimeout(()=>reject(Error('probe did not finish')),1000);timer.unref();})]);
 return {report,calls};
}
test('real probe logic requires a modal, visible, focusable login and matching logged-out health',async()=>{
 const {report,calls}=await probe();
 assert.equal(Object.values(report).filter(value=>value===true).length,5);
 assert.equal(report.nonce,'b'.repeat(64));
 assert.deepEqual(calls.filter(x=>x.path).map(x=>x.path),['/api/health','/api/account']);
 assert.ok(calls.filter(x=>x.path).every(x=>x.init.method===undefined));
 assert.deepEqual(calls.filter(x=>x.command).map(x=>x.command),['get_desktop_info','installed_smoke_report']);
});
for(const [name,configure,field] of [
 ['not modal',o=>{o.dialog.matches=()=>false;},'login'],
 ['disabled',o=>{o.button.disabled=true;},'login'],
 ['invisible',o=>{o.style.visibility='hidden';},'login'],
 ['transparent dialog',o=>{o.dialog.styleOverride={...o.style,opacity:'0'};},'login'],
 ['transparent ancestor',o=>{o.dialog.parentElement={styleOverride:{...o.style,opacity:'0'}};},'login'],
 ['covered',o=>{o.document.elementFromPoint=()=>({});},'login'],
 ['not focusable',o=>{o.button.focus=()=>{};},'login'],
 ['stale health',o=>{o.health.nativeInstanceId='other';},'health'],
 ['already authenticated',o=>{o.account.authenticated=true;o.account.user={id:'synthetic'};},'unauthenticated'],
 ['optional login',o=>{o.account.required=false;},'accountRequired'],
])test(`probe does not certify ${name}`,async()=>{const {report}=await probe({configure});assert.equal(report[field],false);});
test('probe ignores any other origin and waits for DOM readiness',async()=>{
 assert.equal((await probe({origin:'https://external.invalid'})).calls.length,0);
 assert.equal((await probe({loading:true,configure:o=>{o.document.readyState='loading';}})).report.login,true);
});
