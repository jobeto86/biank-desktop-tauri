import {writeFileSync} from 'node:fs';
import {resolve,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const pubkey=String(process.env.BIANK_TAURI_PUBLIC_KEY||'').trim();
if(!pubkey)throw Error('Falta la clave pública de actualización Tauri; no se construye un candidato publicable.');
writeFileSync(resolve(root,'src-tauri/updater.conf.json'),JSON.stringify({bundle:{createUpdaterArtifacts:true},plugins:{updater:{pubkey,endpoints:['https://github.com/jobeto86/biank-desktop/releases/latest/download/latest.json'],windows:{installMode:'quiet'}}}},null,2)+'\n');
console.log('Configuración del canal firmado preparada.');
