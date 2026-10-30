# Idiomatic.sh

Idiomatic.sh exists to improve the developer experience of humans and agents alike.

## Tools

| Tool | Description |
| --- | --- |
| [Asymptote](crates/asymptote/README.md) | Track the state of code quality. Introduce new tools and ratchet down violations over time. |
| [Ryo](crates/ryo/README.md) | Roll your own linter. Build your own linter on top of Ryo. |
| [Tastemaker](crates/tastemaker/README.md) | Codify and enforce architecture. Author custom lint rules and scaffolds for design patterns. |

## Build

Building from a clone on Linux needs the [mold](https://github.com/rui314/mold) linker: `.cargo/config.toml` links with it.

## License

[MIT](LICENSE).
