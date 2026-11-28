# Brief: outbound/store (bead ryo-fmx3.1.13.23.4.14)
Outputs: build/ports/stage2/contracts/outbound/store/sheet.md and api.txt.
Your crate's known facts, with file and line, are in PLAN.md section 8.2 under "Agent A", the paragraph that starts with `store`. Verify each against the code; they are leads. The first draft to check is the Appendix A section for store.
Read build/ports/stage2/contracts/perf-gains.tsv (54 rows, sources checked): every row whose crate column names store is a gain your interface must keep; say on the sheet, row by row, what in your draft keeps it. Read every sheet already written under build/ports/stage2/contracts/ and use their types; never redefine one.
This crate implements the table contract drafted in domain/table (UNPROVEN until the spike). The relations sheet notes five crates encode rows through to_bits/from_bits while the table sheet makes Value non-public: say what the store offers instead.
