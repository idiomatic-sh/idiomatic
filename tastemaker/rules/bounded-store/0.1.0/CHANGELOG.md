# bounded-store 0.1.0

The first release. A pipeline use case, which keeps files across runs, has a lifecycle case in its `use_case_test.rs`,
named with `bounded`, that runs it many times in a row: the rule reports the test file when there is none. Whether the
files stay bounded is shown by running the use case, so the case is the proof and the rule holds that it exists. The
`pipeline` spec places it beside its other lifecycle cases.
