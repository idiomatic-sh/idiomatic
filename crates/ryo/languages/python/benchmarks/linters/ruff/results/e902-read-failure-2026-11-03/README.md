# E902 targeted read-failure proof

Eight complete Full-comparison cases pass raw-exact against both contestants, with zero documented discrepancies or contract-only cases. The current shared collector independently validates all eight identities with errors[]. [Summary](summary.json), [native receipts](cases.jsonl), [offline audit](offline.json) and [invocation](invocation.json) retain the exact evidence.

The built framework is staged tree `bd829a482d39ccad576b22b82979a767d4fdf7fd`, based on HEAD `5331faa5d47c7c9a95ea741e2b763b64c9a48761`; the base HEAD is not mislabeled as the built source. [The source manifest](framework-source-manifest.json) binds all97 framework postimages. The [rule stage](stage-identity.json) comes from clean rule source9e41159b; query, cases and profile bytes were asserted identical to the built source before replay.

Cases cover missing-only and mixed/nested/nonlanguage paths, an unrequested absent dependency, readable Python/text paths, and readable invalid syntax beside a missing input. The latter retains the invalid-syntax diagnostic at byte12 and the missing-file diagnostic at origin0. Unavailable syntax facts remain Unknown; only the automatic frontend projection's applicability excludes sealed missing inputs. Reference diagnostics are not filtered.

This is targeted evidence, not a complete413 inventory result or engine ownership category proof. The [original strict7 failure](initial-strict7/stderr.log), [invocation](initial-strict7/invocation.json) and unchanged [seven original cases](initial-strict7/cases.toml) are retained; its automatic frontend Unknown prompted the explicit readable-input applicability correction. Final narrow source review remains separately owned by integration. No upstream issue opened.
