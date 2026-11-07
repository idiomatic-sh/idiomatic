# Typing origins at the actual annotation visit

The wrapper supplies real AST scopes/read ordinals to type-match-at-check. For postponed annotations it applies Ruff’s module-first forward-reference binding preference before lexical fallback. A concrete module value suppresses a lexical typing match; deletion and annotation-only bindings permit fallback. Import forms preserve direct versus aliased modules.

forward-reference-context models string definitions and postponed ordinary annotation roots. At pinned default settings runtime-required class/decorator configuration is empty. Registered singledispatch implementations still require nonvariadic parameter and return annotations at runtime: the helper resolves the actual register receiver via lookup-attribute and verifies its functools singledispatch/singledispatchmethod decorator. Called register decorators are intentionally excluded, as in the source caller. Real decorator read events supply the lookup point.

RUF020/041 receipts seal this wrapper with the existing RUF051 helper bytes. No existing helper was changed for these rule-specific visit contexts.
