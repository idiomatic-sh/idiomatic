# thin-command

## 0.2.0

The help names a rule's home as the hex's `domain/<concept>/domain.rs`, the domain being
decomposed by concept.

A command's `run` returns `Outcome`: every command
ends as `Outcome { Success, Failure, Error }`, exit 0, 1 and 2: it did its job, it refused
its input or reached a negative verdict, or it could not run. Each CLI owns its `Outcome`
in `src/outcome.rs` (the `outcome` template, which `cli` 0.7.0 places). A function named
`run` in a command file is a finding, "a command's `run` does not return `Outcome`",
unless its return type is a path whose last segment is `Outcome` (`Outcome`,
`crate::outcome::Outcome`). A `run` with no return type, one returning
`std::process::ExitCode`, and one returning `Result<Outcome, _>` are findings; a helper
beside `run` returns what it likes. Every check of 0.1.0 is unchanged.

Read syntactically, like the rest of the rule: the last path segment is compared by name,
so a type alias or another type named `Outcome` passes; which `Outcome` a command may
name is the `cli` spec's import allow-list (`crate::outcome`).

The devtool starter's shared rule block (0.17.0) places it, with the same entry as 0.1.0.

An SDK's functions are held to it too: the shared rule block's entry also reaches
`**/src/entries/**/entry.rs`, the `sdk` spec's files. The rule is unchanged: such a
function is named for its verb, not `run`, so the `Outcome` check never reads it, and it
answers the SDK's own types. Fixtures: passing `thin-entry`, failing `entry-with-logic`
(an `if`, a loop and three service calls).

Moving up from 0.1.0: make each command's `run` return the CLI's `Outcome`
(`crate::outcome::Outcome`, from `cli` 0.7.0's `outcome` template) and have its presenters
return it; the dispatchers convert it to `std::process::ExitCode` once, at `main`
(`impl From<Outcome> for ExitCode`). The fixtures' commands return `Outcome`, so each
failing fixture fails for its own construct only; `returns-exit-code`, `returns-nothing`
and `returns-outcome-in-a-result` fail and `returns-outcome` passes.

## 0.1.0

The first release. It replaces the `thin_command` dylint lint of the
deleted `crates/tastemaker/lints/architecture` library ( the custom lints
become Tastemaker rules defined in TOML). A command parses, calls one service and presents
the result. In every function of a file in scope that declares no trait (closures count as
part of their function), each of these is a finding, reported at the construct: a `loop`,
`while` or `for`; an `if` or `if let`; a `let.. else`; a `match` unless every arm tells
only `Ok` from `Err` (no guard, and a pattern that is `_`, a plain binding, or a
tuple-struct pattern of those) and one arm is `Ok(..)` or `Err(..)`; and a function making
more than one service call, reported once at the function.

The devtool starter's shared rule block (0.6.0) places it:

```toml
{ package = "idiomatic.arch", version = "0.1.0", rule = "thin-command", include = ["**/src/commands/**/command.rs", "**/interface.rs"] },
```

The interfaces a command calls are declared in other crates, so the entry sits at the
product root, where it reaches every `interface.rs`. A file that declares a trait (an
`interface.rs`, whose free functions are the domain's) is read for method names only.

## Differences from the lint

The lint read resolved types; the rule reads syntax:

- **A service call** is a method call whose name is the name of a function declared in an
 in-scope trait named `*Interface`, made on the result of a call (a composition accessor:
 `services.greeting().show(..)`), on a plain `let` binding initialized by a call or method
 call in the same function (`let service = services.greeting(); service.show(..)`), or on
 a parameter whose type names an `*Interface` (`service: &dyn GreetingInterface`); or a
 call whose callee path names an `*Interface` (`GreetingInterface::count(..)`,
 `<T as GreetingInterface>::count(..)`). The lint resolved the method's trait. The receiver
 condition keeps a composition method that shares a service method's name
 (`services.present(..)`, `services.rule()`) from counting: without it, the three products
 gave 50 false findings. Not seen: a service called through UFCS under another name, or
 held in a binding not initialized by a call (a pattern destructuring, a field). A method of
 another type with a service method's name, called on a call's result or on a
 call-initialized binding, counts (stricter).
- **Command code** is every function of a file that declares no trait. A command file that
 declared a trait would go unchecked; the lint checked it.
- **A `match` on a `Result`** is one with an `Ok(..)` or `Err(..)` arm. The lint read the
 scrutinee's type. A `match` whose arms are only `_` and plain bindings, on a non-`Result`,
 no longer counts as a branch (it chooses nothing); every other non-`Result` match still
 does.
- **More than one service call** is one finding at the function, not one per extra call.
- Code inside a macro invocation's arguments is not parsed, so it is not seen; the lint
 skipped only external macros.

## Moving up from the thin_command lint

- Take the devtool starter 0.6.0's shared rule block, or add the entry above to the
 product root's `tastemaker.toml`.
- Drop `--path../tastemaker/lints/architecture` from `cargo dylint`.

## Fixtures

Each `//~ thin_command` marker of the lint's UI fixture is one failing fixture:
`branching/command.rs` gives `match-inspects-result`, `match-guard`, `if`, `if-let`,
`let-else` and `match-not-result`; `looping/command.rs` gives `for`, `while`, `loop` and
`loop-in-closure`; `chatty/command.rs` gives `second-service-call` (a second method call)
and `qualified-service-call` (a second call through `GreetingInterface::count`). The
passing cases pass: `show` (a match on the outcome only), `propagate` (`?`),
`one-service-call` (`chatty` with its first call alone), `presenter` (a presenter beside
the command, outside the entry) and `interface-logic` (the domain's loop and `if` in its
own implementation, outside the entry). Four fixtures go beyond the UI cases and prove the
receiver condition: `injected-service` (two calls on a `&dyn GreetingInterface` parameter)
and `bound-service` (two calls on `let service = services.greeting();`) fail;
`namesake-on-services` (an inherent `Services::count` called on `services` beside one
service call) and `holder-as-argument` (a bound service passed as an argument to such a
method) pass. Every fixture carries the `GreetingInterface` trait in an
`interface.rs` in scope.

## Help that says how to act

Every finding is printed with the rule's help, so the help says what to change and
where, and whether an ignore with a reason is ever legitimate, with the exact ignore
when it is. The rule's text states its reasons itself and cites no design ids.

- The `run`-returns-`Outcome` clause applies to `command.rs` only, so an SDK entry named `run` (`query::run`) is not refused.
