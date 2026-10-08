import {cpSync,existsSync,mkdirSync,mkdtempSync,copyFileSync,chmodSync,readFileSync,writeFileSync,readdirSync,lstatSync,realpathSync,readlinkSync,renameSync,rmSync} from 'node:fs';
import {resolve,dirname,join,relative as pathRelative,isAbsolute} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const desktop=resolve(process.env.BIANK_CORE_DESKTOP||join(root,'../biank/apps/desktop'));
const targetRuntime=join(root,'src-tauri/runtime');
const output=mkdtempSync(join(root,'src-tauri/.runtime-'));
const runtime=join(desktop,'dist/coordinator-runtime');
if(!existsSync(join(runtime,'index.mjs')))throw Error('Construye y prepara el runtime del core antes de empaquetar Tauri.');
if(!existsSync(join(desktop,'dist/web-vite/index.html')))throw Error('Falta el frontend compilado del core.');
mkdirSync(output,{recursive:true});
cpSync(runtime,join(output,'coordinator'),{recursive:true,verbatimSymlinks:true});
cpSync(join(desktop,'dist/web-vite'),join(output,'web'),{recursive:true});
mkdirSync(join(output,'node'),{recursive:true});
const node=join(output,'node',process.platform==='win32'?'node.exe':'node');copyFileSync(process.execPath,node);chmodSync(node,0o755);
for(const name of ['codex','cloudflared']){
 const os=process.platform==='win32'?'win':process.platform==='darwin'?'mac':'linux';
 const source=name==='cloudflared'?join(desktop,'dist',name,`${os}-${process.arch}`):join(desktop,'dist',name);if(!existsSync(source)){if(process.env.BIANK_STAGE_DEVELOPMENT==='1')continue;throw Error(`Falta ${name} empaquetado del core.`);}cpSync(source,join(output,name),{recursive:true});
}
if(!existsSync(join(runtime,'browsers'))&&process.env.BIANK_STAGE_DEVELOPMENT!=='1')throw Error('Falta Chromium empaquetado.');
const clientId=process.env.BIANK_GOOGLE_CLIENT_ID||'';
writeFileSync(join(output,'runtime-config.json'),JSON.stringify({googleIdentity:clientId?{clientId}:null})+'\n');
writeFileSync(join(output,'node-runtime.json'),JSON.stringify({schema:1,platform:process.platform,arch:process.arch,node:process.version,sha256:createHash('sha256').update(readFileSync(node)).digest('hex')})+'\n');
const files={};
function measure(directory,prefix=''){
 for(const name of readdirSync(directory).sort()){
  if(!prefix&&name==='resources-manifest.json')continue;
  const path=join(directory,name),relative=prefix+name,info=lstatSync(path);
  if(info.isSymbolicLink()){
   const target=pathRelative(realpathSync(output),realpathSync(path));if(isAbsolute(target)||target==='..'||target.startsWith('../')||target.startsWith('..\\'))throw Error('Runtime contiene un enlace fuera del paquete.');
   files[relative]={link:readlinkSync(path)};
  }else if(info.isDirectory())measure(path,relative+'/');
  else if(info.isFile())files[relative]={sha256:createHash('sha256').update(readFileSync(path)).digest('hex')};
 }
}
measure(output);
writeFileSync(join(output,'resources-manifest.json'),JSON.stringify({schema:1,files})+'\n');
const previous=join(root,'src-tauri/.runtime-previous');
if(existsSync(previous))throw Error('Existe una preparación anterior pendiente; se conserva.');
if(existsSync(targetRuntime))renameSync(targetRuntime,previous);
try{renameSync(output,targetRuntime);}catch(error){if(existsSync(previous))renameSync(previous,targetRuntime);throw error;}
if(existsSync(previous))rmSync(previous,{recursive:true});
console.log(`Runtime standalone preparado: ${process.platform}/${process.arch}, Node ${process.version}`);
