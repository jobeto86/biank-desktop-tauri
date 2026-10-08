import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {runInNewContext} from 'node:vm';
test('first-party auth bridge forwards to the native OS browser and propagates failure',async()=>{
 const calls=[];let fail=false;
 const window={__TAURI__:{core:{invoke:async(command,args)=>{calls.push({command,args});if(fail)throw 'native string error';}},event:{listen:async()=>()=>{}}}};
 runInNewContext(readFileSync(new URL('./bridge.js',import.meta.url),'utf8'),{window,document:{addEventListener(){}},URL});
 await window.biankDesktop.auth.open('https://accounts.google.com/authorize');
 assert.equal(calls[0].command,'open_external');assert.equal(calls[0].args.url,'https://accounts.google.com/authorize');
 fail=true;await assert.rejects(window.biankDesktop.auth.open('https://auth.openai.com/authorize'),/navegador del sistema/);
});
