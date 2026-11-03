# N999 package scope evidence

These unchanged historical pinned Ruff0.16.5 and CPython3.10.19 observations were preserved before the source correction. The original nested dotted initializer, a valid identifier sibling and an uppercase sibling all produce no N999 under valid outer package. Python importability differs between those names, so import failure alone does not establish a reference defect.

Pinned `invalid_module_name.rs` explicitly selects `package.path().file_name()` for initializer/main files and the file stem for other modules. This is the selected lint policy, not a Python import-equivalence claim. New source controls exercise invalid roots, namespace prefixes, raw dotted roots, main modules and migrations; they await the compiled candidate/reference gate. No original47case source/expectation or raw failure receipt was replaced.
