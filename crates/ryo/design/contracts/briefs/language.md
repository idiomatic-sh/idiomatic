# Brief: outbound/language (bead ryo-fmx3.1.13.23.4.13)
Outputs: build/ports/stage2/contracts/outbound/language/sheet.md and api.txt.
Your crate's known facts, with file and line, are in PLAN.md section 8.2 under "Agent E", the paragraph that starts with `language`. Verify each against the code; they are leads. The first draft to check is the Appendix A section for language.
Read build/ports/stage2/contracts/perf-gains.tsv (54 rows, sources checked): every row whose crate column names language is a gain your interface must keep; say on the sheet, row by row, what in your draft keeps it. Read every sheet already written under build/ports/stage2/contracts/ and use their types; never redefine one.
The relations sheet found keys stated three times (relations' is_key list, the items' declarations, language's KeyedFillItem::KEYS): settle where a language reads them.
