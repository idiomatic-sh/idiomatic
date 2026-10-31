// A builtin introduced in 3.11 remains versioned independently of the query target.
use ryo::languages::contract::{Expected,QualifiedNameExpectedRow,expected_rows,file,name,node,number,span};
const ROWS:&[QualifiedNameExpectedRow]=&[QualifiedNameExpectedRow {
    id:node(2),path:name("builtins.ExceptionGroup"),provenance:name("builtin"),
    since_major:number(3),since_minor:number(11),file:file("qualified_name/input.py"),span:span((1,1),(1,15)),
}];
/// Independently authored source identity, version components and interval.
pub(super) fn expected()->Expected {expected_rows(ROWS)}
