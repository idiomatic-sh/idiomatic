//! Schedule-order resolution (Spike ryo-fmx3): run the file's schedule in order, keep the latest event of each (namespace, name),
//! and at each read or deletion walk Python's lookup path (Language Reference §4.2.2, §7.12, §7.13; PEP 695), one row per namespace
//! that holds an event of the name. Alternatives are linearised in schedule order; no linter policy is applied here.





