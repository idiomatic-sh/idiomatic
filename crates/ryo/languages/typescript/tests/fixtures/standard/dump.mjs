import { readFileSync } from 'node:fs';
import ts from 'typescript';
import { parse, AST_NODE_TYPES, TSError } from '@typescript-eslint/typescript-estree';

const kebab = name => name.replace(/([A-Z]+)([A-Z][a-z])/g, '$1-$2').replace(/([a-z0-9])([A-Z])/g, '$1-$2').toLowerCase();
const encoded = value => `x${Buffer.from(value).toString('hex')}`;
const mask = (1n << 64n) - 1n;
for (const file of process.argv.slice(2).sort()) {
  const bytes = readFileSync(file);
  const source = bytes.toString('utf8');
  let hash = 14695981039346656037n;
  for (const byte of bytes) hash = ((hash ^ BigInt(byte)) * 1099511628211n) & mask;
  const header = `file ${hash.toString(16)} ${bytes.length}`;
  const offset = index => Buffer.byteLength(source.slice(0, index));
  const point = index => {
    const prefix = source.slice(0, index);
    return `${prefix.split('\n').length}:${Buffer.byteLength(prefix.slice(prefix.lastIndexOf('\n') + 1)) + 1}`;
  };
  const span = node => `${point(node.range[0])} ${point(node.range[1])}`;
  let ast;
  try {
    ast = parse(source, { range: true, loc: true, tokens: true, comment: true, jsx: file.endsWith('.tsx'), sourceType: 'module', filePath: file });
  } catch (error) {
    if (!(error instanceof TSError)) throw error;
    console.log(`${header} refused`);
    continue;
  }
  console.log(header);
  const allNodes = [];
  const makeNode = node => {
    const children = [];
    for (const [key, value] of Object.entries(node)) {
      if (['tokens', 'comments', 'loc', 'range', 'parent'].includes(key)) continue;
      for (const item of Array.isArray(value) ? value : [value]) {
        if (item && typeof item === 'object' && typeof item.type === 'string' && item.range) children.push(makeNode(item));
      }
    }
    const tree = { kind: kebab(node.type), from: offset(node.range[0]), to: offset(node.range[1]), children, token: false };
    allNodes.push(node);
    return tree;
  };
  const root = makeNode(ast);
  root.from = 0;
  root.to = bytes.length;
  const attach = (node, token) => {
    const child = node.children.find(child => !child.token && child.from <= token.from && token.to <= child.to);
    if (child) attach(child, token); else node.children.push(token);
  };
  const tokens = [...ast.tokens, ...ast.comments].sort((a, b) => a.range[0] - b.range[0]);
  const hashbang = ts.getShebang(source);
  let end = hashbang ? Buffer.byteLength(hashbang) : 0;
  if (hashbang) attach(root, { kind: 'hashbang', from: 0, to: end, token: true, children: [] });
  for (const token of tokens) {
    const from = offset(token.range[0]);
    const to = offset(token.range[1]);
    if (end < from) attach(root, { kind: 'whitespace', from: end, to: from, token: true, children: [] });
    const suffix = AST_NODE_TYPES[token.type] ? '-token' : '';
    attach(root, { kind: kebab(token.type) + suffix, from, to, token: true, children: [] });
    end = to;
  }
  if (end < bytes.length) attach(root, { kind: 'whitespace', from: end, to: bytes.length, token: true, children: [] });
  const emit = (node, depth) => {
    console.log(`tree ${depth} ${node.token ? 'T' : 'N'} ${node.kind} ${node.from} ${node.to}`);
    node.children.sort((a, b) => a.from - b.from || b.to - a.to);
    for (const child of node.children) emit(child, depth + 1);
  };
  emit(root, 0);
  for (const comment of ast.comments) console.log(`comment ${span(comment)} ${encoded(comment.value)}`);
  const name = node => {
    if (!node) return '?';
    if (node.type === 'Identifier' || node.type === 'PrivateIdentifier') return node.name;
    if (node.type === 'MemberExpression' && !node.computed) {
      const object = name(node.object), property = name(node.property);
      return object !== '?' && property !== '?' ? `${object}.${property}` : '?';
    }
    return '?';
  };
  const field = value => value === '?' ? '?' : encoded(value);
  // A function is a row of `function` when it is declared: a function declaration, or the body of a method, which its key names.
  const methods = new Map();
  for (const node of allNodes) {
    if (['MethodDefinition', 'TSAbstractMethodDefinition'].includes(node.type) || (node.type === 'Property' && (node.method || node.kind !== 'init'))) methods.set(node.value, node.computed ? null : node.key);
  }
  for (const node of allNodes) {
    if (node.type === 'Literal' && typeof node.value === 'string') console.log(`string ${span(node)} ${encoded(node.value)}`);
    if (node.type === 'ImportDeclaration') {
      for (const specifier of node.specifiers) console.log(`import ${span(node)} ${encoded(specifier.local.name)} ${encoded(node.source.value)}`);
      if (!node.specifiers.length) console.log(`import ${span(node)} ? ${encoded(node.source.value)}`);
    }
    if (node.type === 'ImportExpression') console.log(`import ${span(node)} ? ${node.source.type === 'Literal' && typeof node.source.value === 'string' ? encoded(node.source.value) : '?'}`);
    if (node.type === 'CallExpression' && node.callee.type === 'Identifier' && node.callee.name === 'require') console.log(`import ${span(node)} ? ${node.arguments[0]?.type === 'Literal' && typeof node.arguments[0].value === 'string' ? encoded(node.arguments[0].value) : '?'}`);
    if (['FunctionDeclaration', 'FunctionExpression', 'ArrowFunctionExpression', 'TSDeclareFunction', 'TSEmptyBodyFunctionExpression'].includes(node.type)) {
      if (['FunctionDeclaration', 'TSDeclareFunction'].includes(node.type)) console.log(`function ${span(node)} ${field(name(node.id))}`);
      else if (methods.has(node)) console.log(`function ${span(node)} ${field(['Identifier', 'PrivateIdentifier'].includes(methods.get(node)?.type) ? name(methods.get(node)) : '?')}`);
      for (const [ordinal, parameter] of node.params.entries()) {
        let binding = parameter;
        while (['RestElement', 'AssignmentPattern', 'TSParameterProperty'].includes(binding.type)) binding = binding.argument ?? binding.left ?? binding.parameter;
        console.log(`parameter ${span(parameter)} ${parameter.type === 'RestElement' ? 'variadic-positional' : parameter.type === 'Identifier' && parameter.name === 'this' ? 'receiver' : 'positional'} ${ordinal} ${field(name(binding))}`);
      }
    }
    if (node.type === 'CallExpression' || node.type === 'NewExpression') console.log(`call ${span(node.callee)} ${field(name(node.callee))}`);
  }
}
