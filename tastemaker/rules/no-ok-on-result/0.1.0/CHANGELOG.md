# no-ok-on-result 0.1.0

The first release. It reports every `.ok()` with no argument and every `Result::ok` passed as a function: each turns an
error into `None`, so a file that cannot be read or a value that does not decode is treated as absent. The `idiomatic-rust`
spec places it on production code; tests and their data are left out. An ignore with its reason keeps a `.ok()` whose
dropped error is the answer.
