Arm (e) added, the decorator through any trait of the port: an adapter type that implements a trait of a port's `interface.rs`, resolved through the import or the written path, and receives that trait through a generic bound on its `impl` or a parameter of a constructor of its inherent `impl`. Arm (c) is unchanged. Fixtures `decorator-bound`, `decorator-constructor`, `decorator-through-a-bound` and `decorator-through-a-bound-of-two-traits` (violations: the last two the fused-evaluation spike's location and semantic adapters), `a-lookup-holds-its-sources`, `a-port-method-joins-its-workers`, `a-spill-port-method-joins-its-workers` and `a-foreign-trait-shares-a-port-trait-name` (pass).


Arm (c) widened: an adapter names a port trait only in the header of its own `impl <Port> for`; a field, a param, a bound or a `dyn` of any port, its own included, is a finding.

`::crate::` is read as `crate::`: a written path with a leading colon is as local as one without. Fixtures `own-port-only-leading-colon`, `sibling-adapter-path-leading-colon`.
