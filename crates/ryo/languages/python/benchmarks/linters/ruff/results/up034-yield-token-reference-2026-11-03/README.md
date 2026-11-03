# UP034 keyword-token review closure

Pinned Ruff0.16.5 actual direct-file reference checks: canonical Python3.10 19/19 and complete-source Python3.14 counterprofile21/21. Original17case dictionaries are unchanged. New fstring and tstring middle tokens whose text is `yield` remain ordinary literal payload, not yield keywords. Exact source, fullstdout/stderr, actualargv/PID/tool hashes and target settings are preserved in raw-receipts.tar.gz. No candidate replay after this correction has occurred.

The reviewer correctly found an overbroad text predicate, but guessed the exposed token kind incorrectly. The production Python schema explicitly maps TokenKind::Yield to Python-tokenizer-style `name-token`. The corrected blocker is comma OR (name-token AND text yield); the existing real-yield negative stays intact. The new py314 target keeps tstring syntax support explicit rather than widening the defaulttarget.
