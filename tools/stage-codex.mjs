import {cpSync,mkdirSync,chmodSync,existsSync} from 'node:fs';
import {join,resolve,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const desktop=resolve(process.env.BIANK_CORE_DESKTOP||join(root,'../biank/apps/desktop'));
const target=process.platform==='win32'?'x86_64-pc-windows-msvc':process.platform==='darwin'&&process.arch==='arm64'?'aarch64-apple-darwin':null;
if(!target)throw Error('El pipeline de instaladores soporta Windows x64 y macOS ARM64.');
const vendor=join(desktop,'desktop/vendor/node_modules/@openai',process.platform==='win32'?'codex-win32-x64':'codex-darwin-arm64','vendor',target);
const output=join(desktop,'dist/codex');mkdirSync(output,{recursive:true});
for(const name of ['bin','codex-path','codex-resources']){const source=join(vendor,name);if(existsSync(source))cpSync(source,output,{recursive:true});}
const binary=join(output,process.platform==='win32'?'codex.exe':'codex');if(!existsSync(binary))throw Error('Falta el ejecutable Codex fijado por el lockfile.');chmodSync(binary,0o755);
