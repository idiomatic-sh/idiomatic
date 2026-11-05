# Compiler annotation features: language boundary

This fixture explicitly rejects the earlier Ruff-matching provider patch in `rejected-ruff-policy.patch`. Its statement/handler snapshots, relative-future activation, acceptance of future AnnAssign named expressions before3.14, and filename-driven stub rejection were not an independent Python syntax contract. The old Ruff reference receipts remain unchanged as oracle-discrepancy evidence, not language expected results.

Independent `compile(..., dont_inherit=True)` observations from CPython3.13.5 and3.15.0rc2 are retained in cpython313.json/cpython315.json with the script. A source's .py/.pyi filename does not change validity. Generic annotation scopes and native annotation scopes>=3.14 are separate from a module compiler directive; nested lambda bodies are separate scopes, while their defaults retain the annotation restrictions.

One AST-native validation computes future-annotations activation from the legal module prefix: optional first docstring, then absolute eager known future features. Relative imports are ordinary imports. Nested/late/lazy/unknown/malformed future directives do not supply a valid compiler mode. The writer stores the result as a private module property; binding scheduling reads this result rather than scanning arbitrary aliases. Native annotations>=3.14 follow configured language version independently.

References: https://docs.python.org/3.13/reference/simple_stmts.html#future-statements and https://docs.python.org/3.14/reference/executionmodel.html#annotation-scopes . Adapter and binding-schedule tests assert activation, exact syntax ranges, version separation and preserved facts. Existing lexical bindings and qualified-name resolution were audited but left unchanged: they express lexical provenance, not Ruff cached lookup policy.

`exact-ruff-discrepancy.json` records the retained RUF010 source where CPython rejects the walrus but pinned Ruff does not emit invalid-syntax. The provider keeps that real invalidity. RUF010 remains full-report discrepant rather than being force-certified; no benchmark suppression is added.

This source batch awaits integration's coordinated compilation and fresh-worker evidence. Historical worker2ea rule proofs and the rejected provider branch keep their original identities.
