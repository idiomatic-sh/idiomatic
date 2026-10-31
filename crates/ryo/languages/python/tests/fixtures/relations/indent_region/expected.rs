// Independently authored lexical intervals for a single suite: INDENT covers bytes6..10,
// DEDENT is at EOF15. The existing lossless tree places the INDENT after if/name/colon/newline tokens.
use ryo::languages::contract::{Expected,IndentRegionExpectedRow,expected_rows,node,name,number,file,span};
/// Independently specified relation rows for the fixture.
const ROWS:&[IndentRegionExpectedRow]=&[
IndentRegionExpectedRow{id:node(0),parent:node(0),indentation:name(""),start_byte:number(0),end_byte:number(15),file:file("indent_region/input.py"),span:span((1,1),(3,1))},
IndentRegionExpectedRow{id:node(8),parent:node(0),indentation:name("    "),start_byte:number(6),end_byte:number(15),file:file("indent_region/input.py"),span:span((2,1),(3,1))},
];
/// Independently expected root and indented suite.
pub(super) fn expected()->Expected {expected_rows(ROWS)}
