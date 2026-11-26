# idiomatic-sh-ryo-executor

The vector executor: the check's rules lowered to plans (by the engine's lowering) and run over the columns of the walked files, one session for every request of a run. It reads the language contract through the walk and numbered visits, keeps the answers a run may reuse in a store, and replays recorded requests for the benchmarks.

What names an SDK type is not here: the linked entry, the request and report conversions and the call cache stay in the session package above this crate.
