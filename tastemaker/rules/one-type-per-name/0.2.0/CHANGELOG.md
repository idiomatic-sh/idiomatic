# one-type-per-name 0.2.0

A name is not reported when every declaration of it sits in a different language provider (`languages/<provider>/`): providers
never depend on each other and no abstraction spans languages (`Assignment` in the Go and the Python provider). Two
declarations in one provider, or one in a provider and one anywhere else (`RelationName` in the Rust provider and in the
values crate), are still reported.

The rule spans the whole project, not one package: two visible structs or enums with the same name in any two crates the
check covers are reported, with the project's values crate named as the home of the one copy (`RelationName` in ryo's language
and values crates). It also sees the types written `pub struct Name` or `pub enum Name` in the body of a `macro_rules!`
(ryo's `Column`), read as tokens; a `pub(crate)` type or a name given by a metavariable inside a macro is not seen.

Left out as before: private types, `schemas.rs`, `collections/`, `resources/`, an SDK's `src/domain/` (its one edge copy of each shared concept; the SDK API gate forbids re-exporting a core type) (the edges name their types after their verb, and
an SDK's types mirror its core's on purpose), and test code.

# one-type-per-name 0.1.0

The first release: two visible structs or enums of one package with the same name.
