

## Reference discrepancy report

**Reference:** pinned Ruff 0.16.5 (`python-target=3.10`; [profile](../../profiles/F621/benchmark.toml)). **Scope:** Python compiler limit versus pinned reference omission; upstream bug status unverified.

The 12 certified cases below retain their exact source bytes and full intended and observed finding arrays in [cases.toml](cases.toml). The [archived raw receipt](../../results/full-inventory-current/f621-target-supplement/raw-receipts.tar.gz) records an executed worker snapshot; the source and reference-output hashes in each case certificate bind the precise observation. A historical combined receipt may predate a later targeted certificate; follow the certificate identity for that case. Diagnostic spans below are UTF-8 byte offsets.

To reproduce Ruff's reference finding for `256-targets-before-star`, write its exact named file from [cases.toml](cases.toml) into an isolated directory and run the pinned Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`):

```sh
RAYON_NUM_THREADS=1 ruff check --isolated --output-format json --cache-dir ./ruff-cache --select F621 --target-version py310 -- case.py
```

### Documented source-specific difference

Cases (12): `256-targets-before-star`, `257-targets-before-star`, `list-target`, `for-target`, `nested-target`, `parenthesized-boundary-256`, `multiline-punctuation-256`, `utf8-prefix-offset`, `comprehension-target`, `with-target`, `async-for-target`, `unrelated-suppression`.

Representative exact source, `256-targets-before-star` (`case.py`):

```python
x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13, x14, x15, x16, x17, x18, x19, x20, x21, x22, x23, x24, x25, x26, x27, x28, x29, x30, x31, x32, x33, x34, x35, x36, x37, x38, x39, x40, x41, x42, x43, x44, x45, x46, x47, x48, x49, x50, x51, x52, x53, x54, x55, x56, x57, x58, x59, x60, x61, x62, x63, x64, x65, x66, x67, x68, x69, x70, x71, x72, x73, x74, x75, x76, x77, x78, x79, x80, x81, x82, x83, x84, x85, x86, x87, x88, x89, x90, x91, x92, x93, x94, x95, x96, x97, x98, x99, x100, x101, x102, x103, x104, x105, x106, x107, x108, x109, x110, x111, x112, x113, x114, x115, x116, x117, x118, x119, x120, x121, x122, x123, x124, x125, x126, x127, x128, x129, x130, x131, x132, x133, x134, x135, x136, x137, x138, x139, x140, x141, x142, x143, x144, x145, x146, x147, x148, x149, x150, x151, x152, x153, x154, x155, x156, x157, x158, x159, x160, x161, x162, x163, x164, x165, x166, x167, x168, x169, x170, x171, x172, x173, x174, x175, x176, x177, x178, x179, x180, x181, x182, x183, x184, x185, x186, x187, x188, x189, x190, x191, x192, x193, x194, x195, x196, x197, x198, x199, x200, x201, x202, x203, x204, x205, x206, x207, x208, x209, x210, x211, x212, x213, x214, x215, x216, x217, x218, x219, x220, x221, x222, x223, x224, x225, x226, x227, x228, x229, x230, x231, x232, x233, x234, x235, x236, x237, x238, x239, x240, x241, x242, x243, x244, x245, x246, x247, x248, x249, x250, x251, x252, x253, x254, x255, *rest = values
```

- Intended: F621 case.py:0–1431. Pinned Ruff observed: none.
- Basis: Exact-source reference discrepancy: actual CPython 3.10 rejects this single-star store target with too many expressions in star-unpacking assignment, while pinned Ruff 0.16.5 --select F621 emits no diagnostics. Primary evidence: [probe.py](../../profiles/F621/evidence/probe.py), [results.json](../../profiles/F621/evidence/results.json).
- Reproduce: use the [profile](../../profiles/F621/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/full-inventory-current/f621-target-supplement/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.
