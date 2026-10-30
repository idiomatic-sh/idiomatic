# no-machine-paths-shipped 0.1.0

The first version. A path under `/workspaces`, `/tmp`, `/home` or `/Users` in the text of a shipped file is a finding:
Markdown text and code blocks, TOML and JSON values, and comments, wherever the path starts the text or follows a
space, a backtick, a parenthesis, an equals sign or a quote. The string literals of Rust code are left to
`no-hardcoded-paths`.

## Fixtures

`machine-paths-in-docs-and-data` (failing), `relative-and-placeholder-paths` (passing).
