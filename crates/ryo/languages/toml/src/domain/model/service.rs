//! The `model` service: it answers each reader of a file from the file's model, which `documents` works out once when the file is
//! parsed and the engine gives every reader (`Parse::FileModel`): a reader looks its element up, and nothing is scanned again.

