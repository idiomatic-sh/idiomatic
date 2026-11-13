"""Static source oracle for the bounded F811/F841 validity fixtures, not an executor.

Uses Python's AST/tokenizer independently of the provider and queries. It defines
reporting ranges, not CPython's first-error multiplicity. Compiler evidence and
versioned specification evidence establish the validity rules separately.
"""
import ast,io,json,sys,tokenize
from pathlib import Path

def findings(source):
    lines=source.splitlines(keepends=True)
    tokens=list(tokenize.generate_tokens(io.StringIO(source).readline))
    lazy=[t for t in tokens if t.type==tokenize.NAME and t.string=='lazy']
    parsed=source
    if lazy:
        # The recorded case uses one module-prefix lazy import. Remove only the
        # unsupported marker for AST shape; all original token ranges remain authoritative.
        assert len(lazy)==1 and lazy[0].start==(1,0)
        parsed=source[len('lazy '):]
    tree=ast.parse(parsed)
    def offset(position):
        line,col=position
        return sum(len(x.encode()) for x in lines[:line-1])+len(lines[line-1][:col].encode())
    def ast_offset(line,col):
        return sum(len(x.encode()) for x in lines[:line-1])+col+(len('lazy ') if lazy and line==1 else 0)
    result=[]
    for token in lazy:result.append((offset(token.start),offset(token.end),'unsupported-lazy-keyword'))
    def scope(body):
        declarations=[]
        def walk(node):
            if isinstance(node,(ast.FunctionDef,ast.AsyncFunctionDef,ast.ClassDef)):
                scope(node.body)
                return
            if isinstance(node,(ast.Global,ast.Nonlocal)):declarations.append(node)
            for child in ast.iter_child_nodes(node):walk(child)
        for node in body:walk(node)
        global_names={name for node in declarations if isinstance(node,ast.Global) for name in node.names}
        for node in declarations:
            if isinstance(node,ast.Nonlocal):
                for token in tokens:
                    if token.type==tokenize.NAME and node.lineno<=token.start[0]<=node.end_lineno and token.string in node.names and token.string in global_names:
                        result.append((offset(token.start),offset(token.end),'nonlocal-global-conflict'))
    scope(tree.body)
    if lazy and isinstance(tree.body[0], ast.ImportFrom) and tree.body[0].level == 0 and tree.body[0].module == '__future__':
        statement=tree.body[0]
        result.append((offset(lazy[0].start),ast_offset(statement.end_lineno,statement.end_col_offset),'future-lazy-invalid'))
    for node in ast.walk(tree):
        parameters=getattr(node,'type_params',[])
        if parameters:
            start=ast_offset(parameters[0].lineno,parameters[0].col_offset)
            end=ast_offset(parameters[-1].end_lineno,parameters[-1].end_col_offset)
            opening=next(t for t in reversed(tokens) if t.string=='[' and offset(t.start)<start)
            closing=next(t for t in tokens if t.string==']' and offset(t.end)>end)
            result.append((offset(opening.start),offset(closing.end),'unsupported-type-parameter-list'))
    # Compiler features come only from legal absolute module-prefix directives.
    # Relative future imports are ordinary per grammar and CPython's documented
    # fix118267; older CPython's flag activation is an explicitly recorded bug.
    enabled=False
    for index,node in enumerate(tree.body):
        if index==0 and isinstance(node,ast.Expr) and isinstance(node.value,ast.Constant) and isinstance(node.value.value,str):continue
        if isinstance(node,ast.ImportFrom) and node.level==0 and node.module=='__future__' and not lazy:
            enabled |= any(name.name=='annotations' for name in node.names)
        else:break
    if enabled:
        for function in ast.walk(tree):
            if isinstance(function,(ast.FunctionDef,ast.AsyncFunctionDef)):
                annotations=[arg.annotation for arg in [*function.args.posonlyargs,*function.args.args,*function.args.kwonlyargs] if arg.annotation]
                annotations += [arg.annotation for arg in [function.args.vararg,function.args.kwarg] if arg and arg.annotation]
                if function.returns:annotations.append(function.returns)
                for annotation in annotations:
                    for expression in ast.walk(annotation):
                        if isinstance(expression,ast.NamedExpr):result.append((ast_offset(expression.lineno,expression.col_offset),ast_offset(expression.end_lineno,expression.end_col_offset),'named-expression-in-postponed-annotation'))
    return sorted(result)

if __name__=='__main__':
    inputs=json.loads(Path(sys.argv[1]).read_text());output=[]
    for case in inputs:
        if case['rule'] not in ['F811','F841']:continue
        rows=[]
        for filename,source in case['files'].items():
            rows += [dict(rule='invalid-syntax',file=filename,start=a,end=b,validity=kind) for a,b,kind in findings(source)]
        output.append(dict(rule=case['rule'],name=case['name'],files=case['files'],derived=rows))
    print(json.dumps({'oracle_interpreter':sys.version,'scope':'Static AST/token span derivation, not target-runtime proof or a claim about CPython multi-error order','cases':output},indent=2))
