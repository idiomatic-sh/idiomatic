# io-at-boundary 0.1.0

The first release. Our own checks are Tastemaker rules, never clippy
configuration. It replaces these clippy
`disallowed-methods` and `disallowed-types` entries, one concept, the outside world reached
only at its owning external boundary:

- the repository `clippy.toml` and the `clippy-config` template: the `std::fs` functions
 (`read`, `read_to_string`, `write`, `copy`, `rename`, `remove_file`, `remove_dir_all`,
 `create_dir_all`, `read_dir`, `metadata`, `symlink_metadata`, `canonicalize`),
 `std::path::Path::{exists,is_file,is_dir}`, `cap_std::fs::Dir::open_ambient_dir` and
 `std::env::current_dir`;
- the `macro-clippy-config` template: those, and `std::fs::File::{open,create}`,
 `std::process::Command::new`, `std::process::exit`, `std::net::{TcpStream::connect,
 TcpListener::bind, UdpSocket::bind}`, and the types `std::fs::File`,
 `std::process::Command`, `std::net::{TcpStream,TcpListener,UdpSocket}`.

The devtool starter's shared rule block places it with `include = ["**"]` and the boundary
as its `exclude`: `**/adapters/**`, `**/composition.rs`, `**/main.rs`, `**/tests/**`,
`**/benches/**`, `**/*_test.rs` and `**/fixtures.rs`. A macro crate is covered by the same
entry: a macro expands tokens, and a hex domain in it reaches the world only in its adapter.

## Differences from the clippy entries

- The concept is whole where the list was a sample: every path into `std::fs` (types
 included), `tokio::fs`, `tokio::process` and `tokio::net`, `std::process::abort`,
 `env::set_current_dir`, `Path::{try_exists,is_symlink}`, and cap-std's
 `ambient_authority()` count too.
- Clippy resolved each path; the rule matches paths as written. Not seen: a path written
 through a renamed import (`use std::fs as disk;` is not seen at all; `use
 std::process::Command as Run;` is seen at the import only), a path inside a macro
 invocation's tokens, and a filesystem query written with arguments or reached through a
 value the rule cannot type. A filesystem query is a method call with no arguments named
 `exists`, `try_exists`, `is_file`, `is_dir` or `is_symlink`, so the same call on another
 type counts too (a port's `files.exists(&path)` has an argument and never counts).
- An exemption was `#[allow(clippy::disallowed_methods, reason = "...")]` at the owning
 module; no attribute reaches a Tastemaker rule. The boundary is the entry's `exclude`, and
 a single accepted finding elsewhere is a Tastemaker in-source ignore with its reason (E1).

## Moving up from the clippy entries

- Take the devtool starter's shared rule block, or add the entry
 `{ package = "idiomatic.arch", version = "0.1.0", rule = "io-at-boundary", include = ["**"], exclude = ["**/adapters/**", "**/composition.rs", "**/main.rs", "**/tests/**", "**/benches/**", "**/*_test.rs", "**/fixtures.rs"] }`.
- Delete the entries from `clippy.toml` (`clippy-config` 0.3.0) and every
 `#[allow(clippy::disallowed_methods)]` or `#[expect(..)]` of them.

## Fixtures

Failing, one construct each: `fs-call`, `fs-module-import`, `file-open`, `path-query`,
`path-query-qualified`, `working-directory`, `ambient-authority`, `process`, `process-exit`,
`socket`, and `domain-and-adapter` (only the domain's file is a finding). Passing:
`boundary` (an adapter, the composition root, `main.rs`, a test, a bench, a fixtures file
and a `_test.rs` file) and `ports-and-values` (a port's `exists(&path)`, `ExitCode`,
`SocketAddr` and path arithmetic).

## Help that says how to act

Every finding is printed with the rule's help, so the help says what to change and
where, and whether an ignore with a reason is ever legitimate, with the exact ignore
when it is. The rule's text states its reasons itself and cites no design ids.

## The port's name

The help names the port by the suffix the `part` spec gives it, `*Port`, beside a hex crate's `*Interface`. The query is unchanged.
