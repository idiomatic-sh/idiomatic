# Next token

`next-token(tokens=<id: node, file: file>, domain=<id: node, file: file>)` returns, for each supplied token, the first later token of `domain`: `source` is the supplied token, `target` the one found. It walks the provider's `token-link` `next-token` edges, which already skip whitespace, so each token meets only the tokens between it and its answer, never every later token of its file. A token with no later `domain` token has no row.

`code-tokens()` is the domain most callers pass: every lexer token but comments and non-logical newlines. ISC004 finds the token before and after a concatenated string with it, UP034 walks the code token stream, and `decorator-expression-starts` passes the code tokens other than `(` to find the first token of each decorator's expression (PT020, PT025, PT026).
