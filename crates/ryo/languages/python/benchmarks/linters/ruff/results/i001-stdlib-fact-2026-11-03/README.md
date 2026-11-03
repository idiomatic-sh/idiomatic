# Python standard-library membership fact

`catalog-proof.json` binds the provider's versioned `StandardLibraryImport` catalog to CPython's generated `sys.stdlib_module_names` headers for Python 3.10.19 through 3.15.0. Each header initializer and every emitted Rust concat string were parsed independently; the sets match exactly. The local Python 3.13.5 runtime set also matches. Python 3.7–3.9 remain explicitly unverified and yield partial answers.

This proves the catalog data only. I001 query behavior, provider recovery routes, and full Ruff parity remain open.
