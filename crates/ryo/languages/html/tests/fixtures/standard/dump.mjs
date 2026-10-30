import {Parser} from 'parse5';
import {execFileSync} from 'node:child_process';
import {mkdtempSync,readFileSync,writeFileSync,readdirSync,statSync} from 'node:fs';
const root=process.argv[2];
const chromium=process.argv[3];
const browserVersion='Chromium 140.0.7339.16';
if(!chromium || execFileSync(chromium,['--version'],{encoding:'utf8'}).trim()!==browserVersion)throw new Error('Provide pinned Chromium 140.0.7339.16 as the third argument');
const browserSources=[];
function browserDocument(source,mode){
 if(/<noscript\b/i.test(source))throw new Error('The inert browser reference cannot stand for a scripting-enabled noscript document');
 const encoded=JSON.stringify(source).replaceAll('<','\\u003c');
 const script=`const d=new DOMParser().parseFromString(${encoded},'text/html');
 const node=(k,p=[],c=[])=>[k,p.length?JSON.stringify(p):'',c];
 function visit(n){switch(n.nodeType){
 case 9:return node('document',[[7,${JSON.stringify(mode)}]],Array.from(n.childNodes,visit));
 case 10:return node('document-type',[[1,n.name],[5,n.publicId],[6,n.systemId]]);
 case 3:return node('text',[[4,n.data]]);
 case 8:return node('comment',[[4,n.data]]);
 case 11:return node('document-fragment',[],Array.from(n.childNodes,visit));
 case 1:{const children=Array.from(n.attributes,a=>node('attribute',[[1,a.localName],[2,a.namespaceURI??''],[3,a.prefix??''],[4,a.value]]));children.push(...Array.from(n.childNodes,visit));if(n.content)children.push(visit(n.content));return node('element',[[1,n.localName],[2,n.namespaceURI]],children);}
 default:throw new Error('Unexpected native DOM node '+n.nodeType);}}
 document.querySelector('#result').textContent=JSON.stringify(visit(d));`;
 const driver='<pre id="result"></pre><script>'+script+'</script>';
 const args=['--user-data-dir='+mkdtempSync('/tmp/html-reference-chromium-'),'--headless','--no-sandbox','--disable-gpu','--dump-dom','data:text/html,'+encodeURIComponent(driver).replace(/[!'()*]/g,c=>'%'+c.charCodeAt(0).toString(16))];const output=execFileSync(chromium,args,{encoding:'utf8',stdio:['ignore','pipe','ignore']});
 const parsed=Parser.parse(output);let result;
 function find(n){if(n.attrs?.some(a=>a.name==='id'&&a.value==='result'))result=n.childNodes.map(c=>c.value??'').join('');for(const c of n.childNodes??[])find(c);}
 find(parsed);if(!result)throw new Error('Browser did not produce its DOM snapshot');
 browserSources.push(source);return JSON.parse(result);
}
if(!root) throw new Error('usage: node dump.mjs <html-language-root>');
const node=(kind,properties=[],children=[])=>[kind,properties.length?JSON.stringify(properties):'',children];
function visit(n) {
 if(n.nodeName==='#documentType')return node('document-type',[[1,n.name],[5,n.publicId],[6,n.systemId]]);
 if(n.nodeName==='#text')return node('text',[[4,n.value]]);
 if(n.nodeName==='#comment')return node('comment',[[4,n.data]]);
 if(n.nodeName==='#document-fragment')return node('document-fragment',[],(n.childNodes??[]).map(visit));
 const children=n.attrs.map(a=>node('attribute',[[1,a.name],[2,a.namespace??''],[3,a.prefix??''],[4,a.value]]));
 children.push(...(n.childNodes??[]).map(visit));
 if(n.content)children.push(visit(n.content));
 return node('element',[[1,n.tagName],[2,n.namespaceURI]],children);
}
const handlers={onStartTag:'start-tag',onEndTag:'end-tag',onComment:'comment-token',onDoctype:'doctype-token',onCharacter:'characters',onWhitespaceCharacter:'characters',onNullCharacter:'characters'};
function projection(source) {
 const parser=new Parser({sourceCodeLocationInfo:true,scriptingEnabled:true});
 const tokens=[];
 // Intercept only tokenizer delivery, never parser reprocessing of a token.
 parser.tokenizer.handler=new Proxy(parser,{get(target,key){const value=target[key];if(typeof value!=='function')return value;return (...args)=>{if(handlers[key])tokens.push({kind:handlers[key],...args[0].location});return value.apply(target,args);};}});
 parser.tokenizer.write(source,true);
 const lexical=[];let cursor=0;
 for(let i=0;i<tokens.length;i++){
  const token=tokens[i];let end=token.endOffset;
  if(token.kind==='characters')while(tokens[i+1]?.kind==='characters'&&tokens[i+1].startOffset<=end){end=Math.max(end,tokens[++i].endOffset);}
  if(cursor<token.startOffset)lexical.push(['error',source.slice(cursor,token.startOffset),[]]);
  if(cursor<=token.startOffset&&token.startOffset<end)lexical.push([token.kind,source.slice(token.startOffset,end),[]]);
  cursor=Math.max(cursor,end);
 }
 if(cursor<source.length)lexical.push(['error',source.slice(cursor),[]]);
 const document=/<select\b/i.test(source) ? browserDocument(source,parser.document.mode) : node('document',[[7,parser.document.mode]],parser.document.childNodes.map(visit));
 return node('source-file',[],[document,...lexical]);
}
const samples=['','a','<!doctype html>',"<p title='a'>",'<!-- a -->','<template>a</template>',"<p a='x'>",'<p>','<p></p>'];
function collect(dir){for(const name of readdirSync(dir)){const path=dir+'/'+name;if(statSync(path).isDirectory())collect(path);else if(path.endsWith('.html'))samples.push(readFileSync(path,'utf8'));}}
collect(root+'/tests/fixtures');
writeFileSync(root+'/tests/fixtures/standard/parse5.json',JSON.stringify(Object.fromEntries([...new Set(samples)].map(s=>[s,projection(s)])),null,2)+'\n');

writeFileSync(root+'/tests/fixtures/standard/chromium.json',JSON.stringify({version:browserVersion,reason:'parse5 8.0.1 predates customizable select parsing',sources:browserSources},null,2)+'\n');
