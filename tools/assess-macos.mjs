// Record Gatekeeper separately from runtime startup. Ad hoc is not Developer ID.
import {spawnSync} from 'node:child_process';
import {writeFileSync} from 'node:fs';
import {resolve} from 'node:path';
import assert from 'node:assert/strict';
assert.equal(process.platform,'darwin');
const [app,output]=process.argv.slice(2);assert.ok(app&&output);
const path=resolve(app);
const quarantine=spawnSync('xattr',['-w','com.apple.quarantine',`0083;${Math.floor(Date.now()/1000).toString(16)};Chrome;`,path],{encoding:'utf8'});
assert.equal(quarantine.status,0,'Could not prepare browser quarantine assessment');
const result=spawnSync('spctl',['--assess','--type','execute','--verbose=4',path],{encoding:'utf8'});
const detail=(result.stdout||'')+(result.stderr||'');
const status=result.status===0?'accepted':/rejected/i.test(detail)?'rejected':'unavailable';
const report={status,exitCode:result.status,quarantine:true,developerIdNotarization:false,detail};
writeFileSync(output,JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify({gatekeeper:status,quarantine:true,developerIdNotarization:false}));
// Preserve the assessment and never alter Gatekeeper policy or remove quarantine.
