# test-runners-match-rows

## 0.1.0

The first release. A finding is a row of `CASES` whose case word has no `run(CASES, Word::<word>, ..)` call in the file, or such a call whose case word has no row.

A runner over another table (`run(CONTRACT, ..)`) is not seen.
