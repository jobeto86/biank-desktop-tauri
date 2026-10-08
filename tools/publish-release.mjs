// Explicit release ceremony. Build workflows never invoke this automatically.
import {execFileSync} from 'node:child_process';
import {readFileSync,writeFileSync,existsSync,readdirSync} from 'node:fs';
import {resolve,join,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
const repository=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const [version,directory,windowsRun,macosRun,mode]=process.argv.slice(2);
assert.match(version||'',/^\d+\.\d+\.\d+$/,'Explicit stable version required');
assert.ok(directory&&windowsRun&&macosRun,'Usage: publish-release.mjs VERSION ASSETS WINDOWS_RUN MACOS_RUN [--publish]');
assert.ok(!mode||mode==='--publish','Unknown release mode');
assert.equal(JSON.parse(readFileSync(join(repository,'src-tauri/tauri.conf.json'))).version,version);
const assets=resolve(directory),tag=`v${version}`,repo='jobeto86/biank-desktop';
const gh=(args)=>execFileSync('gh',args,{encoding:'utf8',maxBuffer:8*1024*1024});
const required=['Biank-Setup.exe','Biank-Setup.exe.sig','Biank-Setup-arm64.dmg','Biank.app.tar.gz','Biank.app.tar.gz.sig','Biank-Development-Certificate.cer','release-evidence.json'];
for(const name of required)assert.ok(existsSync(join(assets,name)),`Missing ${name}`);
const evidence=JSON.parse(readFileSync(join(assets,'release-evidence.json')));
assert.equal(evidence.version,version);
assert.match(evidence.coreSha,/^[0-9a-f]{40}$/);
for(const [platform,id,expectedSha] of [['Windows',windowsRun,evidence.windows.shellSha],['macOS',macosRun,evidence.macos.shellSha]]){
 assert.match(id,/^\d+$/);
 const run=JSON.parse(gh(['api',`repos/jobeto86/biank-desktop-tauri/actions/runs/${id}`]));
 assert.equal(run.status,'completed');assert.equal(run.head_sha,expectedSha);
 const jobs=JSON.parse(gh(['api',`repos/jobeto86/biank-desktop-tauri/actions/runs/${id}/jobs`])).jobs;
 const job=jobs.find(j=>j.name.includes(platform==='Windows'?'x86_64-pc-windows-msvc':'aarch64-apple-darwin'));
 assert.equal(job?.conclusion,'success',`${platform} native validation failed`);
 for(const name of ['Validate real shared Chromium on the target OS','Verify native vault migration and retained encrypted data',platform==='Windows'?'Install and run actual Windows NSIS candidate':'Mount and run actual macOS DMG candidate'])assert.equal(job.steps.find(s=>s.name===name)?.conclusion,'success',name);
}
for(const [file,sig] of [['Biank-Setup.exe','Biank-Setup.exe.sig'],['Biank.app.tar.gz','Biank.app.tar.gz.sig']]){
 execFileSync(join(repository,'src-tauri/target/debug/examples/verify_update'),[join(assets,file),join(assets,sig),join(process.env.HOME,'.local/share/biank-release-credentials/tauri-updater.key.pub'),version],{stdio:'inherit'});
}
const base=`https://github.com/${repo}/releases/download/${tag}/`;
const win={url:base+'Biank-Setup.exe',signature:readFileSync(join(assets,'Biank-Setup.exe.sig'),'utf8').trim()};
const mac={url:base+'Biank.app.tar.gz',signature:readFileSync(join(assets,'Biank.app.tar.gz.sig'),'utf8').trim()};
writeFileSync(join(assets,'latest.json'),JSON.stringify({version,notes:'Biank Desktop con Tauri y Chromium compartido.',pub_date:new Date().toISOString(),platforms:{'windows-x86_64':win,'windows-x86_64-nsis':win,'darwin-aarch64':mac}},null,2)+'\n');
writeFileSync(join(assets,'SHA256SUMS.txt'),readdirSync(assets).filter(n=>!['SHA256SUMS.txt','release-notes.md'].includes(n)).sort().map(n=>`${createHash('sha256').update(readFileSync(join(assets,n))).digest('hex')}  ${n}\n`).join(''));
if(mode!=='--publish'){console.log('Release prepared and validated; publication requires explicit --publish.');process.exit(0);}
assert.ok(existsSync(join(assets,'release-notes.md')),'Release notes required');
// Existing tags/releases are never overwritten or clobbered by the ceremony.
const files=readdirSync(assets).filter(n=>n!=='release-notes.md').map(n=>join(assets,n));
gh(['release','create',tag,'-R',repo,'--draft','--title',`Biank Desktop ${version} — Tauri`,'--notes-file',join(assets,'release-notes.md'),...files]);
const draft=JSON.parse(gh(['release','view',tag,'-R',repo,'--json','isDraft,assets,url']));
assert.equal(draft.isDraft,true);
for(const name of readdirSync(assets).filter(n=>n!=='release-notes.md'))assert.ok(draft.assets.some(a=>a.name===name&&a.size===readFileSync(join(assets,name)).length),`Draft upload incomplete: ${name}`);
gh(['release','edit',tag,'-R',repo,'--draft=false','--latest']);
const release=JSON.parse(gh(['release','view',tag,'-R',repo,'--json','isDraft,isPrerelease,url']));
assert.equal(release.isDraft,false);assert.equal(release.isPrerelease,false);
const latest=JSON.parse(gh(['api',`repos/${repo}/releases/latest`]));assert.equal(latest.tag_name,tag);
console.log(`Published: ${release.url}`);
