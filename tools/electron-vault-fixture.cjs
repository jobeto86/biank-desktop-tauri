// Synthetic migration fixture only. Never reads a client's store.
const {app,safeStorage}=require('electron');
const {mkdirSync,writeFileSync}=require('node:fs');
const {join,resolve}=require('node:path');
const {randomBytes,createCipheriv}=require('node:crypto');
const root=resolve(process.env.BIANK_MIGRATION_FIXTURE_ROOT||process.argv[2]);
app.setName('Biank');
app.setPath('userData',join(root,'electron-profile'));
app.whenReady().then(()=>{
 if(process.env.BIANK_REQUIRE_PACKAGED_FIXTURE==='1'&&(!app.isPackaged||app.getName()!=='Biank'))throw Error('Packaged Biank identity required');
 if(!safeStorage.isEncryptionAvailable())throw Error('Electron safeStorage unavailable');
 mkdirSync(join(root,'secrets'),{recursive:true});
 const key=randomBytes(32),nonce=randomBytes(12),plain=randomBytes(48);
 const cipher=createCipheriv('aes-256-gcm',key,nonce);
 const encrypted=Buffer.concat([cipher.update(plain),cipher.final(),cipher.getAuthTag()]);
 const sealed=safeStorage.encryptString(key.toString('base64url'));
 writeFileSync(join(root,'secrets/vault.key.sealed'),sealed);
 writeFileSync(join(root,'migration-proof.json'),JSON.stringify({nonce:nonce.toString('base64'),encrypted:encrypted.toString('base64'),expected:require('node:crypto').createHash('sha256').update(plain).digest('hex')}));
 mkdirSync(join(root,'data'),{recursive:true});
 writeFileSync(join(root,'data/history-preserved.txt'),'synthetic conversation history\n');
 key.fill(0);plain.fill(0);
 console.log(JSON.stringify({fixture:'Electron synthetic vault',application:app.getName(),packaged:app.isPackaged,mockKeychain:app.commandLine.hasSwitch('use-mock-keychain'),format:sealed.subarray(0,3).equals(Buffer.from('v10'))?'v10':'other'}));
 app.quit();
}).catch(()=>{console.error('Synthetic Electron vault fixture failed');app.exit(1);});
