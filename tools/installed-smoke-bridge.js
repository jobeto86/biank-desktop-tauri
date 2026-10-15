// Injected only after Rust validates the synthetic CUR160-SMOKE/1 context.
// Read-only UI checks: no login click, OAuth, external browser or secret API.
(()=>{
  const context=__CUR160_SMOKE_CONTEXT__;
  if(location.origin!==context.origin)return;
  const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
  async function inspect(){
    const report={nonce:context.nonce,health:false,login:false,accountRequired:false,unauthenticated:false,bridge:false};
    const get=async route=>{
      const response=await fetch(route,{signal:AbortSignal.timeout(2000),redirect:'error'});
      return response.ok?response.json():null;
    };
    try{
      const [health,account,info]=await Promise.all([get('/api/health'),get('/api/account'),window.__TAURI__.core.invoke('get_desktop_info')]);
      report.health=health?.service==='biank-desktop'&&health.nativeInstanceId===context.engineInstanceId;
      report.accountRequired=account?.required===true;
      report.unauthenticated=account?.authenticated===false&&!account.user;
      report.bridge=info?.edition==='Tauri v2'&&typeof window.biankDesktop?.app?.quit==='function';
      const dialog=document.querySelector('#cbsuiteWelcomeDialog');
      if(dialog?.open&&dialog.matches(':modal')){
        for(const selector of ['#biankGoogleLoginBtn','#cbsuiteGateLoginBtn']){
          const button=dialog.querySelector(selector);if(!button||button.disabled)continue;
          const box=button.getBoundingClientRect();let visible=true;
          for(let element=button;element;element=element.parentElement){
            const style=getComputedStyle(element);
            if(style.display==='none'||style.visibility!=='visible'||Number(style.opacity)===0){visible=false;break;}
          }
          if(box.width<=0||box.height<=0||!visible)continue;
          const x=box.left+box.width/2,y=box.top+box.height/2;
          if(x<0||y<0||x>=innerWidth||y>=innerHeight)continue;
          const hit=document.elementFromPoint(x,y);if(hit!==button&&!button.contains(hit))continue;
          button.focus({preventScroll:true});
          if(document.activeElement===button){report.login=true;break;}
        }
      }
    }catch{/* Only typed booleans cross IPC, never response bodies/errors. */}
    return report;
  }
  async function run(){
    const deadline=Date.now()+60000;let report;
    do{
      report=await inspect();
      if(['health','login','accountRequired','unauthenticated','bridge'].every(key=>report[key]===true))break;
      await sleep(250);
    }while(Date.now()<deadline);
    try{await window.__TAURI__.core.invoke('installed_smoke_report',{report});}catch{}
  }
  if(document.readyState==='loading')addEventListener('DOMContentLoaded',()=>void run(),{once:true});
  else void run();
})();
