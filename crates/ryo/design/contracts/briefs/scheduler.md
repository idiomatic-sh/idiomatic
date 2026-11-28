# Brief: outbound/scheduler (bead ryo-fmx3.1.13.23.4.17)
Outputs: build/ports/stage2/contracts/outbound/scheduler/sheet.md and api.txt.
Your crate's known facts, with file and line, are in PLAN.md section 8.2 under "Agent F", the paragraph that starts with `scheduler`. Verify each against the code; they are leads. The first draft to check is the Appendix A section for scheduler.
Read build/ports/stage2/contracts/perf-gains.tsv (54 rows, sources checked): every row whose crate column names scheduler is a gain your interface must keep; say on the sheet, row by row, what in your draft keeps it. Read every sheet already written under build/ports/stage2/contracts/ and use their types; never redefine one.
Facts checked 28 Nov are in PLAN.md section 6 (the scheduler brief example); parked branch d2502dca4 puts the engine's jobs behind the port.
