# RUF041 — unnecessary-nested-literal

Pinned Ruff0.16.5 `unnecessary_nested_literal.rs`: report the outer Literal when recursively traversing directly nested Literals reaches an ordinary leaf; other subscripts stop traversal, and empty Literals do not trigger a report. Immediate nesting through a tuple suppresses independent nested-root reports. Qualified typing origins accept typing, typing_extensions and _typeshed.

41 cases retain both tagged py/pyi fixtures and full413 oracle arrays. Controls include nested tuples, empty/nonliteral subscripts, quotes, generic and PEP695 shadowing, forward-reference module lookup, module attributes, and singledispatch annotations. Receipt: results/typing-visit-ruf020-ruf041-2026-11-01.json.
