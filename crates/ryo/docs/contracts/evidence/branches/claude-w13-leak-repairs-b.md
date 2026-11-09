# Leak review: claude/w13-leak-repairs-b

Source `f957c0632ef772844030d87c721a064777f82499`, base `b1e81f724206237fad67a42bfe1557fdb838b9e2`. This head changes only the idiomatic-rust 0.1.13 spec and CHANGELOG on top of `3938f4a022`.

## 1 Proof where the contract lives: PASS
Unchanged since `cc762105a8`. The benchmark-owned refusals are proved in `crates/benchmark`, and the adjacency bound in `derived`.

## 2 Every enforcement claim fires: PASS
`tm validate idiomatic-rust@0.1.13` holds. Unplanted, `tm check` over `crates/ryo` gives 18, the existing concept cycles. So the new patterns raise nothing on the 257 provider domain files that import `ryo::languages`, or on their 3,536 `languages::X` paths.

Planted in a provider domain (`languages/python/src/domain/merging/service.rs`), each of these fires:
- the full path `ryo::languages::Parse`, imported or in a signature;
- the trait under another name (`as P`);
- grouped and nested imports;
- the module-relative path after `use ryo::languages;`, in a signature, an `impl` bound or an import;
- the glob `use ryo::languages::*;`, plain or grouped;
- a leading `::`;
- `use ryo as r;` then `r::languages::Parse`.

Both controls stay quiet: `use ryo::languages::ParseRequest;`, and `use ryo::languages;` followed by `languages::ParseRequest`.

One spelling fires 0: `use ryo::languages as l;` then `l::Parse`. I judge it an acceptable residual:
- The 0.1.13 CHANGELOG states it is not covered, and why. The import's target is `ryo::languages` under either binding, so no target pattern can separate it from the allowed plain import, and `*::Parse` gave 98 false positives.
- The tree has 0 uses of it.
- A binding-name check in `import-directions` is filed as an owned follow-up.
- It cannot make any check report a pass.

Check 2 fails only a claim the record makes that does not fire, and the record makes no claim over this spelling.

## 3 Boundaries hold across crates: PASS
No code changed since `cc762105a8`, which compiled with 0 errors.

## 4 One implementation and one representation per concept: PASS
`HandedMatching` stays deleted.

## 5 Layer direction: PASS
No application item crosses crates. `pub mod application` in `execution_domain` exports no item; a private `mod` would do.

## 6 Test seams stay out of production: PASS
No seam.

## Candidates
- The SDK re-export candidate is carried forward as `fixed`.
- The module-reached candidate stays `open`, for the alias form and its follow-up.

VERDICT: PASS
