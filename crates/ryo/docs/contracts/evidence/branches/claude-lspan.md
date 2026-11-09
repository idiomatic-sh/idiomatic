# Leak re-check: claude/lspan at 0371fefcf8

- Source: `0371fefcf817e9b45ccf60d258bdc4e91dd6a3a6`
- Base: `b537645529f840159a35305190bab662c5e35bcf`, the sha reviewed last round
- Scope: the fix for the open finding `fault-catalogue-audits-a-hand-listed-subset-of-variants-abc4eeee`

## Scope of change
The fix is one commit to one file, `execution_domain/src/domain/executable_program/presence/service_test.rs` (+98/-5). It does three things:
- adds `operating_position` and `portable_position`
- completes the instance lists in declaration order
- adds two asserts

No production code changed.

## 1 Proof where the contract lives: PASS
The R6 proof stays in the engine crate. It uses only engine error and fault types, with no product rule set or corpus.

## 2 Every enforcement claim fires: PASS
`errors` now holds all 22 `OperatingError` variants (`operating/errors.rs:8`). `faults` holds all 13 `PortableOperatorFault` variants (`operators/.../contract/types.rs:9`). Both lists follow declaration order. The exhaustive `operating_position` (`service_test.rs:987`) and `portable_position` (`:1014`) are asserted to map each list onto `0..len` (`:1130`, `:1134`).

| Plant | Result |
| --- | --- |
| p3: `SemanticRender` made a SourceMatches row fault with no proof (`:769`; green last round) | **red** at the declaration assert |
| p5: `WrongTest` instance dropped from `errors` (`:1106`) | **red** at `:1130` |
| p6: `UnboundKey` instance dropped from `faults` (`:1124`) | **red** at `:1134` |
| p7 (residual): new `PortableOperatorFault::Planted` with an honest arm in all four forced matches (`operating/service.rs:48`, `service_test.rs:791, :940, :1018`): SourceMatches row fault, no proof, position 13, no instance added | green, as the builder said |

The harness turns no failure into a pass: `run()` asserts that `failed` is empty (`values/src/cases.rs:136`). Every plant was reverted.

## 3 Boundaries hold across crates: PASS
Production visibility is unchanged. The new fns are private to the `cfg(test)` module.

## 4 One implementation and one representation per concept: PASS
The position fns number the variants. They copy no production logic.

## 5 Layer direction: PASS
No production code changed.

## 6 Test seams stay out of production: PASS
There is no new seam. The added code lives only in the `cfg(test)` module (`presence/mod.rs:8`).

## Advisories
- **Residual, advisory by the PM ruling.** Plant p7 confirms the builder's residual. Take a new variant given the next free position, with no instance added: R6 passes it. The position fns cannot know how many variants the enum has, so nothing ties the list length to it. The doc comment above the position fn (`:985`) tells an agent to add the instance. The one plausible honest miss is a batch compile repair that fills the four non-exhaustive arms straight from the cargo JSON without reading that comment. The only mechanical close is a variant count derived on the enum, which needs a new derive in production. That choice is for the TM steward, not a check-2 failure.
- **Carried from the last round.** `located_proof` (`:938`) is a hand-written declaration of which faults each proof covers. R6 cannot tell when a fault sits in the wrong proof arm. The L1 keeps-failure cases are the evidence for those arms.

## Enforcement candidates
- `conditional-failure-declaration-counted-as-unconditional-a9f03941`: fixed.
- `fault-catalogue-audits-a-hand-listed-subset-of-variants-abc4eeee`: fixed (p3, p5 and p6 are red). The appended-variant residual stays on this class for the TM steward to decide.

VERDICT: PASS
