# Brief: inbound/describe (bead ryo-fmx3.1.13.23.4.20)
Outputs: build/ports/stage2/contracts/inbound/describe/sheet.md and api.txt.
Your crate's known facts, with file and line, are in PLAN.md section 8.2 under "Agent G", the paragraph that starts with `describe`. Verify each against the code; they are leads. The first draft to check is the Appendix A section for describe.
Read build/ports/stage2/contracts/perf-gains.tsv (54 rows, sources checked): every row whose crate column names describe is a gain your interface must keep; say on the sheet, row by row, what in your draft keeps it. Read every sheet already written under build/ports/stage2/contracts/ and use their types; never redefine one.
Owner, 28 Nov: the five SDK list and describe calls stay for now; their one product caller is Tastemaker's adapter for tm explain and tm list.
