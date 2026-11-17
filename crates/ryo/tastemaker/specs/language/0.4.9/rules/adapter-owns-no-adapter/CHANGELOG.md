
Arm (c) widened: an adapter names a port trait only in the header of its own `impl <Port> for`; a field, a param, a bound or a `dyn` of any port, its own included, is a finding.

`::crate::` is read as `crate::`: a written path with a leading colon is as local as one without. Fixtures `own-port-only-leading-colon`, `sibling-adapter-path-leading-colon`.
