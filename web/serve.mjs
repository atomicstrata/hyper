import {createServer} from 'node:http';
import {createReadStream} from 'node:fs';
import {stat} from 'node:fs/promises';
import {resolve,extname,sep} from 'node:path';
import {fileURLToPath} from 'node:url';
const directory=fileURLToPath(new URL('./dist/',import.meta.url));
const server=createServer(async(request,response)=>{
  try {
    const url=new URL(request.url,'http://localhost');
    if(url.pathname==='/'){response.writeHead(302,{Location:'/demo/'});response.end();return;}
    let path=resolve(directory,`.${decodeURIComponent(url.pathname)}`);
    if(!path.startsWith(resolve(directory)+sep)){response.writeHead(403);response.end();return;}
    if((await stat(path)).isDirectory())path=resolve(path,'index.html');
    response.setHeader('Content-Type',({'.wasm':'application/wasm','.mjs':'text/javascript','.json':'application/json','.html':'text/html','.css':'text/css'})[extname(path)]??'application/octet-stream');
    createReadStream(path).on('error',()=>{response.destroy();}).pipe(response);
  }catch{response.writeHead(404);response.end('Build the SDK first with npm run build --prefix web.');}
});
server.listen(Number(process.env.HYPER_WEB_PORT??8080),'127.0.0.1',()=>{
  console.log(`Hyper browser demo: http://127.0.0.1:${server.address().port}/demo/`);
});
