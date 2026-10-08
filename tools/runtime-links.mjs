import {cpSync,readdirSync,lstatSync,realpathSync} from 'node:fs';
import {join,relative,isAbsolute} from 'node:path';
function validateSourceLinks(directory,sourceRoot=realpathSync(directory)){
 for(const name of readdirSync(directory)){
  const path=join(directory,name),info=lstatSync(path);
  if(info.isSymbolicLink()){
   const target=relative(sourceRoot,realpathSync(path));
   if(isAbsolute(target)||target==='..'||target.startsWith('../')||target.startsWith('..\\'))throw Error('Fuente contiene un enlace fuera del runtime.');
  }else if(info.isDirectory())validateSourceLinks(path,sourceRoot);
 }
}
export function copyRuntime(source,destination,platform){
 validateSourceLinks(source);
 // A filter selects Node's JS copy path; its native fast path in 22.22.1
 // retained symlinks despite dereference:true in the regression fixture.
 cpSync(source,destination,platform==='darwin'?{recursive:true,dereference:true,filter:()=>true}:{recursive:true,verbatimSymlinks:true});
}
