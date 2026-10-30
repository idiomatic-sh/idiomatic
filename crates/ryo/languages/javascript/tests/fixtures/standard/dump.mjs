import { readFileSync } from 'node:fs';
const { Parser } = await import(process.env.ACORN_MODULE || 'acorn');
const jsx = (await import(process.env.ACORN_JSX_MODULE || 'acorn-jsx')).default;
const parse = (source, options) => Parser.extend(jsx()).parse(source, options);

const kind = name => name.replace(/([A-Z]+)([A-Z][a-z])/g, '$1-$2').replace(/([a-z0-9])([A-Z])/g, '$1-$2').toLowerCase();
const encoded = text => `x${Buffer.from(text).toString('hex')}`;
const samples = JSON.parse(readFileSync(new URL('./samples.json', import.meta.url)));
const inputs = [...process.argv.slice(2).sort().map(path => [path, readFileSync(path)]), ...samples.map(source => ['sample.js', Buffer.from(source)])];
for (const [path, bytes] of inputs) {
  const source = bytes.toString('utf8');
  let hash = 14695981039346656037n;
  for (const byte of bytes) hash = BigInt.asUintN(64, (hash ^ BigInt(byte)) * 1099511628211n);
  const header = `file ${hash.toString(16)} ${bytes.length}`;
  const offset = index => Buffer.byteLength(source.slice(0, index));
  const tokens = [], comments = [];
  let ast;
  try { ast = parse(source, { ecmaVersion: 2025, sourceType: path.endsWith('.cjs') ? 'script' : 'module', onToken: tokens, onComment: comments }); }
  catch { if (path.endsWith('.mjs') || path.endsWith('.cjs')) { console.log(`${header} refused`); continue; } tokens.length=0; comments.length=0; try { ast=parse(source,{ecmaVersion:2025,sourceType:'script',onToken:tokens,onComment:comments}); } catch { console.log(`${header} refused`); continue; } }
  console.log(header);
  const nodes = [];
  function visit(node) {
    nodes.push(node);
    const children = [];
    for (const value of Object.values(node)) {
      for (const child of Array.isArray(value) ? value : [value]) {
        if (child && typeof child === 'object' && typeof child.type === 'string') children.push(visit(child));
      }
    }
    return { kind: kind(node.type), from: offset(node.start), to: offset(node.end), children, token: false };
  }
  const root = visit(ast); root.from = 0; root.to = bytes.length;
  function attach(node, leaf) {
    const child = node.children.find(child => !child.token && child.from <= leaf.from && leaf.to <= child.to);
    if (child) attach(child, leaf); else node.children.push(leaf);
  }
  const lexemes = [];
  for (let index = 0; index < tokens.length; index++) {
    let token = tokens[index];
    if (token.type.label === 'eof') continue;
    const label = token.type.label;
    let category = label === 'name' ? 'identifier-token' : label === 'jsxName' ? 'jsx-identifier-token' : label === 'jsxText' ? 'jsx-text-token' : label === 'privateId' ? 'private-identifier-token' : label === 'num' ? 'numeric' : label === 'string' ? 'string' : label === 'regexp' ? 'regular-expression' : token.type.keyword ? 'keyword' : 'punctuator';
    if ((label === '`' || label === '}') && ['template', 'invalidTemplate'].includes(tokens[index + 1]?.type.label)) {
      token = {start:token.start, end:tokens[index + 2].end}; category = 'template'; index += 2;
    }
    lexemes.push({kind:category, from:offset(token.start), to:offset(token.end), children:[], token:true});
  }
  for (const comment of comments) lexemes.push({kind:kind(comment.type),from:offset(comment.start),to:offset(comment.end),children:[],token:true});
  lexemes.sort((a,b)=>a.from-b.from);
  let end = 0;
  for (const token of lexemes) {
    if (end < token.from) attach(root,{kind:'whitespace',from:end,to:token.from,children:[],token:true});
    attach(root,token); end=token.to;
  }
  if(end < bytes.length) attach(root,{kind:'whitespace',from:end,to:bytes.length,children:[],token:true});
  function emit(node,depth) {
    console.log(`tree ${depth} ${node.token?'T':'N'} ${node.kind} ${node.from} ${node.to}`);
    node.children.sort((a,b)=>a.from-b.from || b.to-a.to);
    for(const child of node.children) emit(child,depth+1);
  }
  emit(root,0);
  const point = index => {const prefix=source.slice(0,index);return `${prefix.split('\n').length}:${Buffer.byteLength(prefix.slice(prefix.lastIndexOf('\n')+1))+1}`;};
  const span = node=>`${point(node.start)} ${point(node.end)}`;
  for(const comment of comments) console.log(`comment ${span(comment)} ${encoded(comment.value)}`);
  const name = node => {
    if (node?.type === 'Identifier') return node.name;
    if (node?.type === 'PrivateIdentifier') return `#${node.name}`;
    if (node?.type === 'ThisExpression') return 'this';
    if (node?.type === 'Super') return 'super';
    if (node?.type === 'MemberExpression' && !node.computed) {
      const object = name(node.object), property = name(node.property);
      if (object !== '?' && property !== '?') return `${object}.${property}`;
    }
    return '?';
  };
  const field = text=>text==='?'?'?':encoded(text);
  // A function is a row of `function` when it is declared: a function declaration, or the body of a method, which its key names.
  const methods = new Map();
  for(const node of nodes) {
    if(node.type==='MethodDefinition' || (node.type==='Property' && (node.method || node.kind!=='init'))) methods.set(node.value, node.computed ? null : node.key);
  }
  for(const node of nodes) {
    if(node.type==='Literal' && typeof node.value==='string') console.log(`string ${span(node)} ${encoded(node.value)}`);
    if(['FunctionDeclaration','FunctionExpression','ArrowFunctionExpression'].includes(node.type)) {
      if(node.type==='FunctionDeclaration') console.log(`function ${span(node)} ${field(name(node.id))}`);
      else if(methods.has(node)) console.log(`function ${span(node)} ${field(['Identifier','PrivateIdentifier'].includes(methods.get(node)?.type) ? name(methods.get(node)) : '?')}`);
      node.params.forEach((parameter,index)=>{const binding=parameter.type==='RestElement'?parameter.argument:parameter.type==='AssignmentPattern'?parameter.left:parameter;console.log(`parameter ${span(parameter)} ${parameter.type==='RestElement'?'variadic-positional':'positional'} ${index} ${field(name(binding))}`);});
    }
    if(node.type==='ImportDeclaration') {
      for(const specifier of node.specifiers) console.log(`import ${span(node)} ${encoded(specifier.local.name)} ${encoded(node.source.value)}`);
      if(!node.specifiers.length) console.log(`import ${span(node)} ? ${encoded(node.source.value)}`);
    }
    if(node.type==='ImportExpression') console.log(`import ${span(node)} ? ${node.source.type==='Literal' && typeof node.source.value==='string' ? encoded(node.source.value) : '?'}`);
    if(node.type==='CallExpression' && node.callee.type==='Identifier' && node.callee.name==='require') console.log(`import ${span(node)} ? ${node.arguments[0]?.type==='Literal' && typeof node.arguments[0].value==='string' ? encoded(node.arguments[0].value) : '?'}`);
    if(node.type==='CallExpression'||node.type==='NewExpression') console.log(`call ${span(node.callee)} ${field(name(node.callee))}`);
  }
}
