# Brief: outbound/cache (bead ryo-fmx3.1.13.23.4.15)
Outputs: build/ports/stage2/contracts/outbound/cache/sheet.md and api.txt.
Your crate's known facts, with file and line, are in PLAN.md section 8.2 under "Agent F", the paragraph that starts with `cache`. Verify each against the code; they are leads. The first draft to check is the Appendix A section for cache.
Read build/ports/stage2/contracts/perf-gains.tsv (54 rows, sources checked): every row whose crate column names cache is a gain your interface must keep; say on the sheet, row by row, what in your draft keeps it. Read every sheet already written under build/ports/stage2/contracts/ and use their types; never redefine one.
CORRECTION to PLAN.md: the small kept store EXISTS: crates/ryo/crates/sdk/src/resources/linter/lint/kept.rs (762 lines) at ba657e79b (branch claude/kept-cache-findings-only; fastapi 840 KB, one edit 301 ms). Read it as the third side, beside MAIN and TIP. Perf rows 2, 10, 14, 53.
