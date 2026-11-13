//! The test data of `linter lint` that fixes queries.

/// The host's placed files, read by a rule.
pub(crate) const HOSTED: &str =
    "let placed = select * from \"host.placed\"; emit placed subject \"path\" key(\"spec\");";
/// Every identifier of the files, which the optional identifier capability promises.
pub(crate) const OPTIONAL_POSITIVE: &str =
    "let names = select * from \"identifier\"; emit names subject file key(span);";
/// A syntactic raw-format outcome requires its advertised optional capability.
pub(crate) const RAW_FORMAT_POSITIVE: &str = "let segments = select * from \"raw-format-segment\" where outcome = \"parsed\"; emit segments subject file key(span);";

/// A syntactic literal outcome requires its advertised optional capability.
pub(crate) const LITERAL_PARSE_POSITIVE: &str = "let parsed = select * from \"literal-expression-parse\" where outcome = \"parsed\"; emit parsed subject file key(span);";
/// Every function of the files.
pub(crate) const FUNCTIONS: &str =
    "let functions = select * from \"function\"; emit functions subject \"file\" key(\"span\");";
/// The functions rule with a misspelled keyword: a rule whose text is wrong.
pub(crate) const MISSPELLED: &str =
    "let functions = selekt * from \"function\"; emit functions subject \"file\" key(\"span\");";

/// The files with no identifier named `missing`: a proven absence of an optional relation's rows.
pub(crate) const OPTIONAL_ABSENCE: &str = "let names = select * from \"identifier\" where name = \"missing\"; let absent = antijoin(left=\"syntax-tree\",right=names,left-on=[\"file\"],right-on=[\"file\"]); emit absent subject file key(span);";
