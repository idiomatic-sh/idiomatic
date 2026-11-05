---
name: systemic-audit
description: Audit repeated architecture or performance findings across their full implementation family before claiming ownership or closure.
---

# Systemic audit

- State the violated invariant behind a finding before changing its one caller.
- Enumerate the full family from the authoritative enum, catalog, dispatch table and implementation sites, including optimized and native routes.
- Choose the compiler-visible boundary first: crate dependency, public contract and private implementation access. Keep engine ownership explicit.
- Enforce structural placement and imports with Tastemaker; prove route semantics with the shared runtime harness where structure cannot establish them.
- Apply the check to the entire family; prove a deliberately introduced violation is rejected and a new implementation cannot silently escape coverage.
- Record remaining routes, failed checks and source-bound evidence in the existing ownership matrix and batch report. A repaired caller alone never closes a family.
