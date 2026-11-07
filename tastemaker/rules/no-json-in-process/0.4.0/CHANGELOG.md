# no-json-in-process 0.4.0

Two readings of JSON as input are no longer in process. A package that holds an adapter's `schemas.rs`
(`outbound/<port>/adapters/<adapter>/schemas.rs`) is a boundary package whatever spec places the file: it decodes the wire shape of an
outside file, a language's `package.json` reader among them. An optional dependency, which only a feature (the `fixtures` feature's
test oracle) enables, is no part of a build that names no feature and is not reported. A normal, non-optional dependency of a package
that holds no boundary, and any source file that names `serde_json` outside a boundary, are reported as before.
