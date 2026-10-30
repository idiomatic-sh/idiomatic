# Idiomatic

Developer tools built in Rust.

| Tool | Purpose |
| --- | --- |
| [Ryo](crates/ryo/README.md) | SDK for repository-wide linters: typed facts, relational queries, evaluation and caching. |
| [Tastemaker](crates/tastemaker/README.md) | Authors and validates rules, scaffolds and specs; checks repositories through a Ryo adapter. |
| [Asymptote](crates/asymptote/README.md) | Tracks accepted lint violations so existing debt can decrease without silently increasing. |

Tastemaker owns authored policy and generation. Ryo owns analysis and execution.
Asymptote is a separate tool for managing violations.

Development commands are in [CONTRIBUTING.md](CONTRIBUTING.md);
architecture design is in [ARCHITECTURE.md](ARCHITECTURE.md).

## License

[MIT](LICENSE).
