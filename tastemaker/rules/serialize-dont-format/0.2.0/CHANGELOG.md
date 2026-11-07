# serialize-dont-format 0.2.0

A raw string with real newlines is read line by line even when it quotes code in backticks: Tastemaker's `INIT_WORKSPACE`
(a workspace file written from text, whose description quotes `tm check --fix`) was skipped by the backtick exclusion that
is meant for one-line messages. The exclusion now applies to a literal with no real newline. New fixture
`raw-document-with-backticks`.

# serialize-dont-format 0.1.0

The first version. A string literal of the code that is a fragment of a TOML or JSON document is a finding: a
`[table]` header line, a `key = {value}` pair (at the start of the text, after a newline, a comma or a brace), an
inline table opened with `{{ key = `, a `key = [{` array, or a JSON member opened after a brace or a comma. The document is built as its
type and written by its serializer instead. A message that quotes code in backticks is left out.
