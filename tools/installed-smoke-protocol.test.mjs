import test from 'node:test';
import assert from 'node:assert/strict';
import {parseReceipt,rootDigest,verifyVault,verifyUi,verifyShutdown,SmokeError} from './installed-smoke-protocol.mjs';
const expected={root:'/synthetic/root',runId:'a'.repeat(32),nonce:'b'.repeat(64),boot:1,candidateSha:'c'.repeat(40)};
const receipt=(kind='vault')=>({schemaVersion:1,kind,runId:expected.runId,nonce:expected.nonce,boot:1,candidateSha:expected.candidateSha,rootSha256:rootDigest(expected.root),engineInstanceId:'engine-first-00000001',enginePid:41});
const first=()=>({...receipt(),operation:'put',digestSha256:'d'.repeat(64)});
const secondExpected={...expected,boot:2,nonce:'e'.repeat(64)};
const second=()=>({...first(),nonce:secondExpected.nonce,boot:2,operation:'resolve',engineInstanceId:'engine-second-0000002',enginePid:42});
const ui=()=>({...receipt('ui'),shellPid:20,health:true,login:true,accountRequired:true,unauthenticated:true,bridge:true});
const shutdown=()=>({...receipt('shutdown'),shellPid:20,drained:true,engineExited:true});
const invalid=fn=>assert.throws(fn,error=>error instanceof SmokeError&&error.message==='CUR160_RECEIPT_INVALID');

test('two vault receipts require a fresh engine and resolution of the original digest',()=>{
 const one=verifyVault(JSON.stringify(first()),expected);
 assert.deepEqual(verifyVault(JSON.stringify(second()),secondExpected,one),second());
 for(const change of [{digestSha256:'f'.repeat(64)},{enginePid:41},{engineInstanceId:one.engineInstanceId},{operation:'put'},{nonce:one.nonce}])invalid(()=>verifyVault(JSON.stringify({...second(),...change}),secondExpected,one));
 invalid(()=>verifyVault(JSON.stringify(second()),secondExpected));
});
test('closed identity rejects stale, foreign, malformed and extra evidence',()=>{
 for(const change of [{schemaVersion:'1'},{kind:'ui'},{runId:'f'.repeat(32)},{nonce:'f'.repeat(64)},{boot:2},{candidateSha:'f'.repeat(40)},{rootSha256:'f'.repeat(64)},{enginePid:0},{enginePid:1.5},{engineInstanceId:'short'},{digestSha256:null},{secret:'must-not-be-echoed'}]){
  invalid(()=>verifyVault(JSON.stringify({...first(),...change}),expected));
 }
 for(const field of Object.keys(first())){const value=first();delete value[field];invalid(()=>verifyVault(JSON.stringify(value),expected));}
});
test('UI and shutdown require matching process identity and literal positive checks',()=>{
 assert.deepEqual(verifyUi(JSON.stringify(ui()),expected,first(),20),ui());
 assert.deepEqual(verifyShutdown(JSON.stringify(shutdown()),expected,first(),20),shutdown());
 for(const key of ['health','login','accountRequired','unauthenticated','bridge'])for(const value of [false,1,'true',null])invalid(()=>verifyUi(JSON.stringify({...ui(),[key]:value}),expected,first(),20));
 for(const value of [{shellPid:21},{enginePid:42},{engineInstanceId:'other-engine-0000001'}])invalid(()=>verifyUi(JSON.stringify({...ui(),...value}),expected,first(),20));
 for(const key of ['drained','engineExited'])invalid(()=>verifyShutdown(JSON.stringify({...shutdown(),[key]:false}),expected,first(),20));
 invalid(()=>verifyShutdown(JSON.stringify(shutdown()),expected,first(),21));
});
test('parser rejects duplicated decoded keys, nested payloads, BOM and excessive size without echo',()=>{
 for(const raw of ['{"nonce":"old","nonce":"new"}','{"nonce":1,"no\\u006ece":2}','{"data":{"secret":"hidden"}}','{"data":[1]}','\ufeff{}','null','[]','{',JSON.stringify({data:'x'.repeat(4096)})])invalid(()=>parseReceipt(raw));
 assert.deepEqual(parseReceipt('{ "path" : "C:\\\\tmp\\\\value\\\":x", "ready":true, "pid":42 }'),{path:'C:\\tmp\\value\":x',ready:true,pid:42});
});
