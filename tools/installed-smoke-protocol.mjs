import {createHash} from 'node:crypto';

export class SmokeError extends Error {
  constructor(code) { super(`CUR160_${code}`); this.code=code; }
}
const reject=()=>{throw new SmokeError('RECEIPT_INVALID');};
const hex=(value,length)=>typeof value==='string'&&new RegExp(`^[a-f0-9]{${length}}$`).test(value);
const pid=value=>Number.isSafeInteger(value)&&value>0;
export const rootDigest=root=>createHash('sha256').update(root,'utf8').digest('hex');

// Receipts are flat, closed objects. JSON.parse alone silently accepts duplicate
// keys; reject them before using any field as evidence.
export function parseReceipt(raw) {
  if(typeof raw!=='string'||Buffer.byteLength(raw)>4096||raw.charCodeAt(0)===0xfeff)reject();
  let value;try{value=JSON.parse(raw);}catch{reject();}
  if(!value||Array.isArray(value)||typeof value!=='object')reject();
  let i=0;const keys=new Set();
  const space=()=>{while(/\s/.test(raw[i]??'')&&i<raw.length)i++;};
  const string=()=>{
    const start=i++;while(i<raw.length){const c=raw[i++];if(c==='\\'){i++;continue;}if(c==='"')return JSON.parse(raw.slice(start,i));}
    reject();
  };
  space();if(raw[i++]!=='{')reject();space();
  while(raw[i]!=='}'){
    if(raw[i]!=='"')reject();const key=string();if(keys.has(key))reject();keys.add(key);
    space();if(raw[i++]!==':')reject();space();
    if(raw[i]==='"')string();else{const start=i;while(i<raw.length&&raw[i]!==','&&raw[i]!=='}')i++;if(/[\[{}\]]/.test(raw.slice(start,i)))reject();}
    space();if(raw[i]===','){i++;space();continue;}if(raw[i]!=='}')reject();
  }
  return value;
}
const identityKeys=['schemaVersion','kind','runId','nonce','boot','candidateSha','rootSha256','engineInstanceId','enginePid'];
function identity(value,expected,kind,extra) {
  const keys=[...identityKeys,...extra];
  if(Object.keys(value).length!==keys.length||keys.some(key=>!Object.hasOwn(value,key)))reject();
  if(value.schemaVersion!==1||value.kind!==kind||value.runId!==expected.runId||!hex(value.runId,32)||
    value.nonce!==expected.nonce||!hex(value.nonce,64)||value.boot!==expected.boot||![1,2].includes(value.boot)||
    value.candidateSha!==expected.candidateSha||!hex(value.candidateSha,40)||value.rootSha256!==rootDigest(expected.root)||
    !pid(value.enginePid)||typeof value.engineInstanceId!=='string'||!/^[A-Za-z0-9_-]{16,128}$/.test(value.engineInstanceId))reject();
}
export function verifyVault(raw,expected,first) {
  const value=parseReceipt(raw);identity(value,expected,'vault',['operation','digestSha256']);
  if(value.operation!==(expected.boot===1?'put':'resolve')||!hex(value.digestSha256,64))reject();
  if(expected.boot===2&&(!first||value.digestSha256!==first.digestSha256||value.enginePid===first.enginePid||value.engineInstanceId===first.engineInstanceId||value.nonce===first.nonce))reject();
  return value;
}
export function verifyUi(raw,expected,vault,shellPid) {
  const value=parseReceipt(raw);identity(value,expected,'ui',['shellPid','health','login','accountRequired','unauthenticated','bridge']);
  if(!pid(shellPid)||value.shellPid!==shellPid||value.enginePid!==vault.enginePid||value.engineInstanceId!==vault.engineInstanceId||
    ['health','login','accountRequired','unauthenticated','bridge'].some(key=>value[key]!==true))reject();
  return value;
}
export function verifyShutdown(raw,expected,vault,shellPid) {
  const value=parseReceipt(raw);identity(value,expected,'shutdown',['shellPid','drained','engineExited']);
  if(value.shellPid!==shellPid||!pid(shellPid)||value.enginePid!==vault.enginePid||value.engineInstanceId!==vault.engineInstanceId||value.drained!==true||value.engineExited!==true)reject();
  return value;
}
