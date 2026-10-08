// Appended only to an explicitly isolated debug webview, never a release.
addEventListener('DOMContentLoaded',()=>setTimeout(async()=>{try{
 const info=await window.__TAURI__.core.invoke('get_desktop_info');
 const health=await fetch('/api/health').then(r=>r.ok?r.json():null);
 await window.__TAURI__.core.invoke('qa_report',{state:{bridge:info.edition==='Tauri v2',health:health?.service==='biank-desktop',login:!!document.querySelector('#cbsuiteWelcomeDialog[open]')}});
}catch{}},5000));

addEventListener('keydown',event=>{if(event.ctrlKey&&event.shiftKey&&event.key.toLowerCase()==='q'){event.preventDefault();window.__TAURI__.core.invoke('request_exit').catch(()=>{});}});
