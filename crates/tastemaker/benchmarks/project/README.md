# Repository query benchmark

The reference workload is the pinned Tastemaker release running `tm check --format sarif` in each CI workspace. The coordinator's `tm-bench/scripts` captures SDK calls and their answers in paired JSON lines. The installed binary, workload checkout, CPU and memory measurement protocol and SARIF hashes are recorded with that capture.

Build the Ryo CLI's `vec-readers` Cargo bench. From this directory, run the scope controls and replay a captured check with the resulting executable.

```sh
python3 check_scope.py "$VEC_READERS" "$SHIPPING_READERS"
python3 replay.py original-ryo.trace.gz "$VEC_READERS" project-ryo.tsv
```

`check_scope.py` compares positive and empty results against the shipping SDK. It also exercises missing and partial populations. `replay.py` covers every captured invocation, including those with a partial reference. It passes the captured query text, parameters, complete inventory, rule admission and typed host rows into one process. It reverses file visitation and table rows on the second evaluation. The TSV records complete typed equality, a mismatch, an order change, a refusal or a partial reference separately for each rule invocation. A receipt hashes sorted rows with all columns, the subject and the ordered key. Expected rows never enter evaluation. An optional fourth argument writes the forward-only input stream for timing a fresh executor process.

This benchmark exercises consumers of captured data. A whole-check gate also needs live placement, observation producers, suppressions and a byte comparison of the resulting SARIF.
