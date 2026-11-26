// Only the explicitly authorized read-only catalog uses a larger budget.
import {AuthError,require} from './protocol.mjs';
import {strictJson,CATALOG_MAX_BYTES} from './strict-json.mjs';
export const AUTH_MAX_BYTES=256*1024;
export async function readJsonBody(body,modelCatalog=false){
  const max=modelCatalog===true?CATALOG_MAX_BYTES:AUTH_MAX_BYTES;
  const prefix=modelCatalog===true?'catalog':'auth';
  require(body&&typeof body[Symbol.asyncIterator]==='function',prefix+'_response_missing');
  let n=0;const chunks=[];
  for await(const b of body){
    require(b instanceof Uint8Array,prefix+'_response_invalid');
    n+=b.byteLength;require(n<=max,prefix+'_response_limit');
    chunks.push(Buffer.from(b));
    // Avoid retaining millions of tiny chunk objects on pathological streams.
    if(chunks.length>=1024){const joined=Buffer.concat(chunks,n);chunks.length=0;chunks.push(joined);}
  }
  try{
    return strictJson(new TextDecoder('utf-8',{fatal:true}).decode(Buffer.concat(chunks,n)),modelCatalog);
  }catch(e){
    const code=['json_limit','json_structure_limit','json_duplicate_key'].includes(e?.message)?e.message:'json_invalid';
    throw new AuthError(prefix+'_'+code);
  }
}
