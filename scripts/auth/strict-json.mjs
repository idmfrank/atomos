// Native JSON grammar validation plus a bounded duplicate-key audit.
// No prototype mutation; audit escaped keys by their decoded strings.
export const CATALOG_MAX_BYTES=8*1024*1024;
export function strictJson(text,modelCatalog=false){
  const maxBytes=modelCatalog===true?CATALOG_MAX_BYTES:512*1024;
  const maxValues=modelCatalog===true?262144:16384;
  if(typeof text!=='string'||Buffer.byteLength(text)>maxBytes)throw Error('json_limit');
  const value=JSON.parse(text);
  let i=0,count=0;
  const ws=()=>{while(i<text.length&&' \t\r\n'.includes(text[i]))i++;};
  function string(){
    const start=i++;
    while(i<text.length){
      const c=text[i++];
      if(c==='"')return JSON.parse(text.slice(start,i));
      if(c==='\\')i++;
    }
    throw Error('json_string');
  }
  function walk(depth){
    if(depth>32||++count>maxValues)throw Error('json_structure_limit');
    ws();
    const c=text[i];
    if(c==='{'){
      i++;ws();const keys=new Set();
      if(text[i]==='}'){i++;return;}
      for(;;){
        if(text[i]!=='"')throw Error('json_key');
        const k=string();
        if(keys.has(k))throw Error('json_duplicate_key');
        keys.add(k);ws();if(text[i++]!==':')throw Error('json_colon');
        walk(depth+1);ws();if(text[i]==='}'){i++;return;}
        if(text[i++]!==',')throw Error('json_comma');ws();
      }
    }
    if(c==='['){
      i++;ws();if(text[i]===']'){i++;return;}
      for(;;){walk(depth+1);ws();if(text[i]===']'){i++;return;}if(text[i++]!==',')throw Error('json_comma');}
    }
    if(c==='"'){string();return;}
    while(i<text.length&&!(' \t\r\n,]}'.includes(text[i])))i++;
  }
  walk(0);ws();if(i!==text.length)throw Error('json_trailing');
  return value;
}
