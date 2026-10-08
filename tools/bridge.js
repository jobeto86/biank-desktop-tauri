// Runs only in the first-party Biank webview. External Chromium pages receive no bridge.
(() => {
 const invoke=(command,args={})=>window.__TAURI__.core.invoke(command,args);
 const subscribe=(event,callback)=>{let stop=null,disposed=false;window.__TAURI__.event.listen(event,e=>callback(e.payload)).then(unlisten=>{if(disposed)unlisten();else stop=unlisten;});return()=>{disposed=true;stop?.();};};
 window.biankBrowser={native:false,external:true,action:payload=>invoke('browser_action',{payload}),onState:()=>()=>{}};
 window.biankDesktop={
  auth:{open:url=>invoke('open_external',{url}).catch(()=>{throw new Error('No se pudo abrir el navegador del sistema. Inténtalo de nuevo.');})},
  legacyImport:{select:kind=>invoke('import_select',{kind}),apply:id=>invoke('import_apply',{id})},
  app:{focus:()=>invoke('focus_main'),about:()=>invoke('show_about'),quit:()=>invoke('request_exit')},
  titleBar:{setTheme:theme=>invoke('set_theme',{theme})},
  view:{zoom:action=>invoke('zoom_view',{action})},
  updates:{getState:()=>invoke('update_state'),check:()=>invoke('check_update'),apply:()=>invoke('apply_update'),onState:cb=>subscribe('biank-updates:state',cb)},
  pets:{mode:'in-app'},
 };
 document.addEventListener('click',event=>{const link=event.target.closest?.('a[href]');if(!link)return;const url=new URL(link.href,location.href);if(url.origin!==location.origin&&['http:','https:'].includes(url.protocol)){event.preventDefault();invoke('open_external',{url:url.href}).catch(()=>{});}});
})();
