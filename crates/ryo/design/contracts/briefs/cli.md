# Brief: edges/cli (bead ryo-fmx3.1.13.23.4.22)
Outputs: build/ports/stage2/contracts/edges/cli/sheet.md and api.txt.
Your crate's known facts, with file and line, are in PLAN.md section 8.2 under "Agent G", the paragraph that starts with `cli`. Verify each against the code; they are leads. The first draft to check is the Appendix A section for cli.
Read build/ports/stage2/contracts/perf-gains.tsv (54 rows, sources checked): every row whose crate column names cli is a gain your interface must keep; say on the sheet, row by row, what in your draft keeps it. Read every sheet already written under build/ports/stage2/contracts/ and use their types; never redefine one.
The folder is today crates/ryo/cli; the package is idiomatic-sh-ryo.
