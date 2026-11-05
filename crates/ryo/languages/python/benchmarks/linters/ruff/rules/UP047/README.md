# UP047

Reports a module-level function signature when its parameter annotations reference recognized standalone type variables. Return annotations alone, quoted annotations, nested functions, methods, existing PEP 695 parameters and defaulted type variables are excluded. Repeated references produce one diagnostic per signature. AnyStr conversion requires unshadowed builtin names for its constraints.

The query uses existing semantic syntax, binding-use, qualified-name and language-target facts. It emits diagnostics only. Type-variable lookup follows explicit binding identities; literal-string embedded type reads are distinguished from physical expression reads.

The canonical profile targets Python 3.12, the first version supporting PEP 695. The Python 3.10 countertarget preserves the same source files and retains target-specific syntax diagnostics rather than treating every source as an empty negative. Suppression remains in the canonical shared helper.

`cases.toml` preserves every pinned Ruff 0.16.5 upstream fixture for this rule, plus independent exact-span, alias, shadowing, nesting, default, suppression and existing-parameter controls. Expectation origins and full pinned outputs are retained with the source-bound proof. Passing this fixture set does not certify the safety of Ruff's generated fixes.

Sources: [pinned rule implementation](https://github.com/astral-sh/ruff/blob/9e4938c4a60bed3e87a11ee1e1db1bd23f4d964a/crates/ruff_linter/src/rules/pyupgrade/rules/pep695/non_pep695_generic_function.rs), [PEP 695](https://peps.python.org/pep-0695/).
