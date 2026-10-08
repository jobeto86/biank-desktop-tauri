import {cpSync,readdirSync,lstatSync,realpathSync,readlinkSync} from 'node:fs';
import {join,relative,isAbsolute} from 'node:path';
function validateSourceLinks(directory,sourceRoot=realpathSync(directory)){
 for(const name of readdirSync(directory)){
  const path=join(directory,name),info=lstatSync(path);
  if(info.isSymbolicLink()){
   const target=relative(sourceRoot,realpathSync(path));
   if(isAbsolute(target)||target==='..'||target.startsWith('../')||target.startsWith('..\\'))throw Error('Fuente contiene un enlace fuera del runtime.');
   if(isAbsolute(readlinkSync(path)))throw Error('Fuente contiene un enlace absoluto; el paquete exige enlaces relativos.');
  }else if(info.isDirectory())validateSourceLinks(path,sourceRoot);
 }
}
export function copyRuntime(source,destination,platform){
 validateSourceLinks(source);
 cpSync(source,destination,{recursive:true,verbatimSymlinks:true});
}
