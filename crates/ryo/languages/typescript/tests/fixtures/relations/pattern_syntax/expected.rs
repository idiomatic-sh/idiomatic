// The rows `relations/pattern_syntax/input.ts` must give.

use ryo::languages::contract::{
    Expected, PatternSyntaxExpectedRow, expected_rows, file, name, node, number, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[PatternSyntaxExpectedRow] = &[
    PatternSyntaxExpectedRow { id: node(77), carrier: node(10), kind: name("pattern"), text: name("^a|b$"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((1, 12), (1, 17)) },
    PatternSyntaxExpectedRow { id: node(78), carrier: node(10), kind: name("disjunction"), text: name("^a|b$"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((1, 12), (1, 17)) },
    PatternSyntaxExpectedRow { id: node(79), carrier: node(10), kind: name("alternative"), text: name("^a"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((1, 12), (1, 14)) },
    PatternSyntaxExpectedRow { id: node(80), carrier: node(10), kind: name("boundary-assertion"), text: name("^"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((1, 12), (1, 13)) },
    PatternSyntaxExpectedRow { id: node(81), carrier: node(10), kind: name("character"), text: name("a"), value: number(97), file: file("pattern_syntax/input.ts"), span: span((1, 13), (1, 14)) },
    PatternSyntaxExpectedRow { id: node(82), carrier: node(10), kind: name("alternative"), text: name("b$"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((1, 15), (1, 17)) },
    PatternSyntaxExpectedRow { id: node(83), carrier: node(10), kind: name("character"), text: name("b"), value: number(98), file: file("pattern_syntax/input.ts"), span: span((1, 15), (1, 16)) },
    PatternSyntaxExpectedRow { id: node(84), carrier: node(10), kind: name("boundary-assertion"), text: name("$"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((1, 16), (1, 17)) },
    PatternSyntaxExpectedRow { id: node(85), carrier: node(23), kind: name("pattern"), text: name("(?=x)(?<n>c)+\\k<n>\\1(?:d)."), value: number(0), file: file("pattern_syntax/input.ts"), span: span((2, 12), (2, 38)) },
    PatternSyntaxExpectedRow { id: node(86), carrier: node(23), kind: name("disjunction"), text: name("(?=x)(?<n>c)+\\k<n>\\1(?:d)."), value: number(0), file: file("pattern_syntax/input.ts"), span: span((2, 12), (2, 38)) },
    PatternSyntaxExpectedRow { id: node(87), carrier: node(23), kind: name("alternative"), text: name("(?=x)(?<n>c)+\\k<n>\\1(?:d)."), value: number(0), file: file("pattern_syntax/input.ts"), span: span((2, 12), (2, 38)) },
    PatternSyntaxExpectedRow { id: node(88), carrier: node(23), kind: name("look-around-assertion"), text: name("(?=x)"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((2, 12), (2, 17)) },
    PatternSyntaxExpectedRow { id: node(89), carrier: node(23), kind: name("disjunction"), text: name("x"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((2, 15), (2, 16)) },
    PatternSyntaxExpectedRow { id: node(90), carrier: node(23), kind: name("alternative"), text: name("x"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((2, 15), (2, 16)) },
    PatternSyntaxExpectedRow { id: node(91), carrier: node(23), kind: name("character"), text: name("x"), value: number(120), file: file("pattern_syntax/input.ts"), span: span((2, 15), (2, 16)) },
    PatternSyntaxExpectedRow { id: node(92), carrier: node(23), kind: name("quantifier"), text: name("(?<n>c)+"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((2, 17), (2, 25)) },
    PatternSyntaxExpectedRow { id: node(93), carrier: node(23), kind: name("capturing-group"), text: name("(?<n>c)"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((2, 17), (2, 24)) },
    PatternSyntaxExpectedRow { id: node(94), carrier: node(23), kind: name("disjunction"), text: name("c"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((2, 22), (2, 23)) },
    PatternSyntaxExpectedRow { id: node(95), carrier: node(23), kind: name("alternative"), text: name("c"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((2, 22), (2, 23)) },
    PatternSyntaxExpectedRow { id: node(96), carrier: node(23), kind: name("character"), text: name("c"), value: number(99), file: file("pattern_syntax/input.ts"), span: span((2, 22), (2, 23)) },
    PatternSyntaxExpectedRow { id: node(97), carrier: node(23), kind: name("named-reference"), text: name("\\k<n>"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((2, 25), (2, 30)) },
    PatternSyntaxExpectedRow { id: node(98), carrier: node(23), kind: name("indexed-reference"), text: name("\\1"), value: number(1), file: file("pattern_syntax/input.ts"), span: span((2, 30), (2, 32)) },
    PatternSyntaxExpectedRow { id: node(99), carrier: node(23), kind: name("ignore-group"), text: name("(?:d)"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((2, 32), (2, 37)) },
    PatternSyntaxExpectedRow { id: node(100), carrier: node(23), kind: name("disjunction"), text: name("d"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((2, 35), (2, 36)) },
    PatternSyntaxExpectedRow { id: node(101), carrier: node(23), kind: name("alternative"), text: name("d"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((2, 35), (2, 36)) },
    PatternSyntaxExpectedRow { id: node(102), carrier: node(23), kind: name("character"), text: name("d"), value: number(100), file: file("pattern_syntax/input.ts"), span: span((2, 35), (2, 36)) },
    PatternSyntaxExpectedRow { id: node(103), carrier: node(23), kind: name("dot"), text: name("."), value: number(0), file: file("pattern_syntax/input.ts"), span: span((2, 37), (2, 38)) },
    PatternSyntaxExpectedRow { id: node(104), carrier: node(36), kind: name("pattern"), text: name("[^\\d\\p{L}a-z]\\b"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((3, 12), (3, 27)) },
    PatternSyntaxExpectedRow { id: node(105), carrier: node(36), kind: name("disjunction"), text: name("[^\\d\\p{L}a-z]\\b"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((3, 12), (3, 27)) },
    PatternSyntaxExpectedRow { id: node(106), carrier: node(36), kind: name("alternative"), text: name("[^\\d\\p{L}a-z]\\b"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((3, 12), (3, 27)) },
    PatternSyntaxExpectedRow { id: node(107), carrier: node(36), kind: name("character-class"), text: name("[^\\d\\p{L}a-z]"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((3, 12), (3, 25)) },
    PatternSyntaxExpectedRow { id: node(108), carrier: node(36), kind: name("character-class-escape"), text: name("\\d"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((3, 14), (3, 16)) },
    PatternSyntaxExpectedRow { id: node(109), carrier: node(36), kind: name("unicode-property-escape"), text: name("\\p{L}"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((3, 16), (3, 21)) },
    PatternSyntaxExpectedRow { id: node(110), carrier: node(36), kind: name("character-class-range"), text: name("a-z"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((3, 21), (3, 24)) },
    PatternSyntaxExpectedRow { id: node(111), carrier: node(36), kind: name("character"), text: name("a"), value: number(97), file: file("pattern_syntax/input.ts"), span: span((3, 21), (3, 22)) },
    PatternSyntaxExpectedRow { id: node(112), carrier: node(36), kind: name("character"), text: name("z"), value: number(122), file: file("pattern_syntax/input.ts"), span: span((3, 23), (3, 24)) },
    PatternSyntaxExpectedRow { id: node(113), carrier: node(36), kind: name("boundary-assertion"), text: name("\\b"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((3, 25), (3, 27)) },
    PatternSyntaxExpectedRow { id: node(114), carrier: node(49), kind: name("pattern"), text: name("[\\q{ab}]"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((4, 12), (4, 20)) },
    PatternSyntaxExpectedRow { id: node(115), carrier: node(49), kind: name("disjunction"), text: name("[\\q{ab}]"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((4, 12), (4, 20)) },
    PatternSyntaxExpectedRow { id: node(116), carrier: node(49), kind: name("alternative"), text: name("[\\q{ab}]"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((4, 12), (4, 20)) },
    PatternSyntaxExpectedRow { id: node(117), carrier: node(49), kind: name("character-class"), text: name("[\\q{ab}]"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((4, 12), (4, 20)) },
    PatternSyntaxExpectedRow { id: node(118), carrier: node(49), kind: name("class-string-disjunction"), text: name("\\q{ab}"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((4, 13), (4, 19)) },
    PatternSyntaxExpectedRow { id: node(119), carrier: node(49), kind: name("class-string"), text: name("ab"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((4, 16), (4, 18)) },
    PatternSyntaxExpectedRow { id: node(120), carrier: node(49), kind: name("character"), text: name("a"), value: number(97), file: file("pattern_syntax/input.ts"), span: span((4, 16), (4, 17)) },
    PatternSyntaxExpectedRow { id: node(121), carrier: node(49), kind: name("character"), text: name("b"), value: number(98), file: file("pattern_syntax/input.ts"), span: span((4, 17), (4, 18)) },
    PatternSyntaxExpectedRow { id: node(122), carrier: node(68), kind: name("pattern"), text: name("\\u{1F44D}"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((5, 23), (5, 32)) },
    PatternSyntaxExpectedRow { id: node(123), carrier: node(68), kind: name("disjunction"), text: name("\\u{1F44D}"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((5, 23), (5, 32)) },
    PatternSyntaxExpectedRow { id: node(124), carrier: node(68), kind: name("alternative"), text: name("\\u{1F44D}"), value: number(0), file: file("pattern_syntax/input.ts"), span: span((5, 23), (5, 32)) },
    PatternSyntaxExpectedRow { id: node(125), carrier: node(68), kind: name("character"), text: name("\\u{1F44D}"), value: number(128077), file: file("pattern_syntax/input.ts"), span: span((5, 23), (5, 32)) },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
