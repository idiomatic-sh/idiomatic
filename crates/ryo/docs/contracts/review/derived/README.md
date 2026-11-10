# Derived ownership repair checkpoint

Source checkpoint `a374f2ab09ef03f4884a78b51d70726286a2b8f5` (tree `81219f3e1f1dd0c891fb2e342d19a810baeae5b9`) follows `fec5c83c7f` on integration base `36b0cdbea1`.

The batch moves mutable index, selection and graph-retention implementations under `DerivedOwner`; consumers receive opaque leased graph/domain handles and borrowed data. Raw storage/payload allocation getters remain confined to that owner. Graph demand accepts issued handles and checks live owner/parent provenance. Held immutable leases remain readable after retirement, while new admission and graph demands reject retired provenance. The MembershipValues route binds its actual single semantic source while preserving demanded operand admission order. Selector positions are published with their required dependencies inside the common owner.

The real-source Rust harness retains exact diagnostic-code/span checks and adds function-item uses to every inline probe module, including authorized controls. New controls reject detached storage/payload/graph/domain allocations and forged graph demand/fields; authorized borrowed reads remain explicit. A production-owner test checks retirement reads, rejected demands, retained required charge and final release.

Status: **uncompiled and unrun**. No author Cargo, rustc or performance run. Existing runtime297 evidence predates this repair. `rustfmt` and `git diff --check` pass. Installed TM validates the existing layer rule and strengthened program boundary (exact qualified compile, aliased descriptor, corrected pure-domain controls); the final scoped source check still reports **142 findings, zero suppressed**. No layer/program-boundary/directory-layout/closed-placement finding remains in that scope. Remaining findings include legacy/moved type/style/cycle debt and SelectionFixture's test-only gating versus the fixtures-feature placement requirement. These are unresolved findings, not an acceptance waiver.

Root owns combined compilation and runtime validation. Preserve the existing all-13 acquisition, raw lifetime/error and generation/source/descriptor tests; no output-correctness or performance closure is claimed.
