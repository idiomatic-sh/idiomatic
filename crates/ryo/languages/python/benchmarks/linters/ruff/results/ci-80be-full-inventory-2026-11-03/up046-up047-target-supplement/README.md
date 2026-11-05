# PEP 695 migration rule fixture proof

The existing sealed worker verifies the complete UP046 and UP047 fixtures at Python 3.12 and Python 3.10. The source-bound controls retain all five pinned upstream fixture files and independent alias, shadowing, nesting, default, quoted-annotation, suppression and exact-span controls. The Python 3.10 controls preserve all source files, including unsupported-syntax diagnostics. Rule-specific diagnostics are target gated.

Only external RyoQL, cases and profile registration were added; no language/provider/engine changes or generated fixes are included. The first native query mismatches are retained. Corrected queries use the public type-params field spelling and exclude embedded quoted type expressions from the physical-expression visitor policy.

The existing generic supplement validator checks current source, settings, cases, helpers and actual native attestations. These offline summaries derive from preserved verify-cases receipts, not a collector invocation or combined413 rerun. The original exact80be full report and all previous summaries remain unchanged. Reference execution accounting remains explicitly unavailable under the established diagnostics contract; candidate completeness remains mandatory.
