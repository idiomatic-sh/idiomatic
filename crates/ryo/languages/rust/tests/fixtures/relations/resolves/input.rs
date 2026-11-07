use crate::types::Record;

struct Stored<T> { record: Record, items: Vec<Record>, extra: T, plain: std::string::String, local: Local }

struct Local;
