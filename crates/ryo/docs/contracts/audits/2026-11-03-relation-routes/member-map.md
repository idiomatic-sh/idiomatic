# 65-member route map

The 650 exact provider cells and anchors are in `members.json`; common owner/proof details are in `owner-routes.md`. `R` means an actual declared reader, `N/A` a typed semantic absence assertion, `U` declared NotYetRead. Optional promise gating is recorded independently in JSON; it can turn a declared N/A into effective unavailability, marked effective U in three cells below. All six manifest-provenance rows are produced per file but consumed only by CrossFile plans; per-file G/T, K, L/LC, N, J and C are N/A for those inputs. No runtime proof is inferred.

| Member | Class / provenance | Reader providers | Declared N/A providers | Declared U providers | Native / direct Node operands |
| --- | --- | --- | --- | --- | --- |
| Function | Code / syntax | go, javascript, python, rust, typescript | css:NoSuchConstruct, html:NoSuchConstruct, json:NotCode, markdown:NoSuchConstruct, toml:NotCode | — | N/A; id |
| Annotates | Code / syntax | go, python, rust, typescript | css:NoSuchConstruct, html:NoSuchConstruct, javascript:NoSuchConstruct, json:NotCode, markdown:NoSuchConstruct, toml:NotCode | — | N/A; occurrence,owner |
| Assigns | Code / syntax | go, javascript, python, rust, typescript | css:NoSuchConstruct, html:NoSuchConstruct, json:NotCode, markdown:NoSuchConstruct, toml:NotCode | — | N/A; occurrence,binding,value,owner |
| Comment | Lexical / syntax | css, go, html, javascript, json, markdown, python, rust, toml, typescript | — | — | N/A; id |
| Directive | Literal / syntax | go, javascript, typescript | css:NoSuchConstruct, html:NoSuchConstruct, json:NoSuchConstruct, markdown:NoSuchConstruct, python:NoSuchConstruct, rust:NoSuchConstruct, toml:NoSuchConstruct | — | N/A; N/A (no Node) |
| Decorates | Code / syntax | python, rust, typescript | css:NoSuchConstruct, go:NoSuchConstruct, html:NoSuchConstruct, javascript:NoSuchConstruct, json:NotCode, markdown:NoSuchConstruct, toml:NotCode | — | N/A; occurrence,declaration |
| Defines | Code / syntax | go, html, javascript, markdown, python, rust, typescript | css:NoSuchConstruct, json:NotCode, toml:NotCode | — | N/A; id,owner |
| Visibility | Code / syntax | javascript, rust, typescript | css:NoSuchConstruct, go:NoSuchConstruct, html:NoSuchConstruct, json:NotCode, markdown:NoSuchConstruct, python:NoSuchConstruct, toml:NotCode | — | N/A; occurrence,declaration |
| Inherits | Code / syntax | go, javascript, python, rust, typescript | css:NoSuchConstruct, html:NoSuchConstruct, json:NotCode, markdown:NoSuchConstruct, toml:NotCode | — | N/A; occurrence,declaration |
| Parameter | Code / syntax | go, javascript, python, rust, typescript | css:NoSuchConstruct, html:NoSuchConstruct, json:NotCode, markdown:NoSuchConstruct, toml:NotCode | — | N/A; id,owner |
| Reference | Code / syntax | css, go, html, javascript, markdown, python, rust, typescript | json:NotCode, toml:NotCode | — | N/A; occurrence,owner |
| Mention | Code / syntax | rust | css:NoSuchConstruct, go:NoSuchConstruct, html:NoSuchConstruct, javascript:NoSuchConstruct, json:NotCode, markdown:NoSuchConstruct, python:NoSuchConstruct, toml:NotCode, typescript:NoSuchConstruct | — | N/A; occurrence,owner |
| Scope | Code / syntax | go, html, javascript, markdown, python, rust, typescript | css:NoSuchConstruct, json:NotCode, toml:NotCode | — | N/A; parent,child |
| StaticCall | Code / syntax | go, javascript, python, rust, typescript | css:NoSuchConstruct, html:NoSuchConstruct, json:NotCode, markdown:NoSuchConstruct, toml:NotCode | — | N/A; occurrence,owner |
| CallArgument | Code / syntax | go, javascript, python, rust, typescript | css:NoSuchConstruct, html:NoSuchConstruct, json:NotCode, markdown:NoSuchConstruct, toml:NotCode | — | N/A; call,expression |
| Resolves | Code / syntax | go, python, rust, typescript | css:NoSuchConstruct, html:NoSuchConstruct, javascript:NoSuchConstruct, json:NotCode, markdown:NoSuchConstruct, toml:NotCode | — | N/A; occurrence,owner,definition |
| BindingEvent | Code / syntax | python | — | css, go, html, javascript, json, markdown, rust, toml, typescript | N/A; occurrence,scope,phase |
| BindingRead | Code / syntax | python | — | css, go, html, javascript, json, markdown, rust, toml, typescript | N/A; reference,scope,phase |
| BindingUse | Code / syntax | python | — | css, go, html, javascript, json, markdown, rust, toml, typescript | N/A; reference,binding,scope,phase |
| BindingShadow | Code / syntax | python | — | css, go, html, javascript, json, markdown, rust, toml, typescript | N/A; binding,previous,scope,previous_scope,phase |
| ImplicitSymbol | Code / syntax | python | — | css, go, html, javascript, json, markdown, rust, toml, typescript | N/A; scope |
| QualifiedName | Code / syntax | python | html:NoSuchConstruct (effective U), json:NotCode, markdown:NoSuchConstruct, toml:NotCode | css, go, javascript, rust, typescript | N/A; id |
| Identifier | Code / syntax | python | json:NotCode, toml:NotCode | css, go, html, javascript, markdown, rust, typescript | N/A; id |
| ExpressionShape | Code / syntax | python | json:NotCode (effective U), toml:NotCode (effective U) | css, go, html, javascript, markdown, rust, typescript | N/A; id,shape |
| LiteralProperties | Code / syntax | python | — | css, go, html, javascript, json, markdown, rust, toml, typescript | N/A; id |
| FunctionTerminal | Code / syntax | python | — | css, go, html, javascript, json, markdown, rust, toml, typescript | N/A; id |
| ExpressionType | Code / syntax | python | — | css, go, html, javascript, json, markdown, rust, toml, typescript | N/A; id |
| InterpolatedStringLiteral | Code / syntax | python | — | css, go, html, javascript, json, markdown, rust, toml, typescript | N/A; id,owner |
| LanguageTarget | Engine / syntax | python | — | css, go, html, javascript, json, markdown, rust, toml, typescript | N/A; N/A (no Node) |
| CompilerFeature | Code / syntax | python | — | css, go, html, javascript, json, markdown, rust, toml, typescript | N/A; N/A (no Node) |
| ConstantKey | Code / syntax | python | — | css, go, html, javascript, json, markdown, rust, toml, typescript | N/A; id,key |
| FormatParse | Code / syntax | python | — | css, go, html, javascript, json, markdown, rust, toml, typescript | N/A; id |
| FormatField | Code / syntax | python | — | css, go, html, javascript, json, markdown, rust, toml, typescript | N/A; id |
| BraceFormatParse | Code / syntax | python | — | css, go, html, javascript, json, markdown, rust, toml, typescript | N/A; id |
| BraceFormatField | Code / syntax | python | — | css, go, html, javascript, json, markdown, rust, toml, typescript | N/A; id |
| RawFormatSegment | Code / syntax | python | — | css, go, html, javascript, json, markdown, rust, toml, typescript | N/A; id,segment |
| RawFormatField | Code / syntax | python | — | css, go, html, javascript, json, markdown, rust, toml, typescript | N/A; id,segment |
| LiteralExpressionParse | Code / syntax | python | — | css, go, html, javascript, json, markdown, rust, toml, typescript | N/A; id |
| StaticImport | Code / syntax | css, go, javascript, python, rust, typescript | html:NoSuchConstruct, json:NotCode, markdown:NoSuchConstruct, toml:NotCode | — | N/A; occurrence,owner |
| StringLiteral | Literal / syntax | css, go, html, javascript, json, markdown, python, rust, toml, typescript | — | — | N/A; occurrence,owner |
| BytesLiteral | Literal / syntax | python | — | css, go, html, javascript, json, markdown, rust, toml, typescript | N/A; occurrence,owner |
| FormatSpecification | Code / syntax | python | — | css, go, html, javascript, json, markdown, rust, toml, typescript | N/A; id,origin |
| SyntaxChild | Structure / syntax | css, go, html, javascript, json, markdown, python, rust, toml, typescript | — | — | Tree; parent,child |
| SyntaxError | Structure / syntax | css, go, html, javascript, json, markdown, python, rust, toml, typescript | — | — | N/A; id |
| SyntaxTree | Structure / syntax | css, go, html, javascript, json, markdown, python, rust, toml, typescript | — | — | Tree; id |
| SemanticSyntax | Structure / syntax | css, go, html, javascript, json, markdown, python, rust, toml, typescript | — | — | N/A; id |
| SemanticChild | Structure / syntax | css, go, html, javascript, json, markdown, python, rust, toml, typescript | — | — | N/A; parent,child |
| EmbeddedExpression | Code / syntax | python | — | css, go, html, javascript, json, markdown, rust, toml, typescript | N/A; root,host |
| SourceLocation | Structure / syntax | css, go, html, javascript, json, markdown, python, rust, toml, typescript | — | — | N/A; id |
| IndentRegion | Lexical / syntax | python | css:NoSuchConstruct, go:NoSuchConstruct, html:NoSuchConstruct, javascript:NoSuchConstruct, json:NoSuchConstruct, markdown:NoSuchConstruct, rust:NoSuchConstruct, toml:NoSuchConstruct, typescript:NoSuchConstruct | — | N/A; id,parent |
| Mapping | Keyed / syntax | go, json, toml | css:NotAKeyedDocument, html:NoSuchConstruct, javascript:NotAKeyedDocument, markdown:NoSuchConstruct, python:NotAKeyedDocument, rust:NotAKeyedDocument, typescript:NotAKeyedDocument | — | N/A; id,owner |
| Entry | Keyed / syntax | go, json, toml | css:NotAKeyedDocument, html:NoSuchConstruct, javascript:NotAKeyedDocument, markdown:NoSuchConstruct, python:NotAKeyedDocument, rust:NotAKeyedDocument, typescript:NotAKeyedDocument | — | N/A; occurrence,owner |
| SequenceElement | Keyed / syntax | json, toml | css:NotAKeyedDocument, go:NoSuchConstruct, html:NoSuchConstruct, javascript:NotAKeyedDocument, markdown:NoSuchConstruct, python:NotAKeyedDocument, rust:NotAKeyedDocument, typescript:NotAKeyedDocument | — | N/A; occurrence,owner |
| Prose | Document / syntax | html, markdown | css:NotADocument, go:NotADocument, javascript:NotADocument, json:NotADocument, python:NotADocument, rust:NotADocument, toml:NotADocument, typescript:NotADocument | — | N/A; id,owner |
| NumericLiteral | Code / syntax | python | — | css, go, html, javascript, json, markdown, rust, toml, typescript | N/A; id |
| TokenLink | Lexical / syntax | python | — | css, go, html, javascript, json, markdown, rust, toml, typescript | N/A; source,target |
| PackageDependency | Manifest / manifest | go, javascript, python, rust, typescript | css:NoManifests, html:NoManifests, json:NoManifests, markdown:NoManifests, toml:NoManifests | — | N/A; N/A (no Node) |
| WorkspaceMember | Manifest / manifest | go, javascript, python, rust, typescript | css:NoManifests, html:NoManifests, json:NoManifests, markdown:NoManifests, toml:NoManifests | — | N/A; N/A (no Node) |
| SourceDependency | Manifest / manifest | go, javascript, python, rust, typescript | css:NoManifests, html:NoManifests, json:NoManifests, markdown:NoManifests, toml:NoManifests | — | N/A; N/A (no Node) |
| AnalyzedSource | Manifest / manifest | go, javascript, python, rust, typescript | css:NoManifests, html:NoManifests, json:NoManifests, markdown:NoManifests, toml:NoManifests | — | N/A; N/A (no Node) |
| FilePackage | Manifest / manifest | go, javascript, python, rust, typescript | css:NoManifests, html:NoManifests, json:NoManifests, markdown:NoManifests, toml:NoManifests | — | N/A; N/A (no Node) |
| ModuleIdentity | Manifest / manifest | python | — | css, go, html, javascript, json, markdown, rust, toml, typescript | N/A; N/A (no Node) |
| FilesystemFile | Engine / engine | N/A—all ten; common engine producer | — | — | N/A; N/A (no Node) |
| FilesystemLineCount | Engine / engine | N/A—all ten; common engine producer | — | — | N/A; N/A (no Node) |
| FilesystemExecutable | Engine / engine | N/A—all ten; common engine producer | — | — | N/A; N/A (no Node) |
