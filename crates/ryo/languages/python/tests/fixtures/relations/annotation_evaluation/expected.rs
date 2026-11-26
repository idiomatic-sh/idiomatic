// The rows `relations/annotation_evaluation/input.py` must give, written by hand from the relation's definition and the source, with
// each element named by its place in the walk's table (the nodes in source order, then the tokens): the module opens with the annotations
// future, so every annotation of a parameter, a return and a class variable is postponed (kept as text) in the lazy mode; the annotation
// of a local variable is never evaluated (Language Reference 7.2.2: annotations are not evaluated in a function scope), also lazy; the
// elements of an annotation are its expression nodes and its tokens (no whitespace), and the string annotation's own expression
// (`Result`, nodes 101 and 102) has an evaluation of its own and no row here.
//   `a: int`            root 11 (Name), elements 11 and its token 48
//   `*rest: str`        root 14 (Name), elements 14 and its token 55
//   `-> "Result"`       root 15 (Constant), elements 15, its string part 16 and its token 60
//   `inner: list[int]`  root 19 (Subscript), elements 19, 20 and token 67 (`list`), `[` 68, 21 and token 69 (`int`), `]` 70
//   `count: int`        root 29 (Name), elements 29 and its token 94

use ryo::languages::contract::{AnnotationEvaluationExpectedRow, Expected, expected_rows, file, name, node};

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(&[
        AnnotationEvaluationExpectedRow {
            id: node(11),
            root: node(11),
            evaluation: name("postponed"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
        AnnotationEvaluationExpectedRow {
            id: node(48),
            root: node(11),
            evaluation: name("postponed"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
        AnnotationEvaluationExpectedRow {
            id: node(14),
            root: node(14),
            evaluation: name("postponed"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
        AnnotationEvaluationExpectedRow {
            id: node(55),
            root: node(14),
            evaluation: name("postponed"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
        AnnotationEvaluationExpectedRow {
            id: node(15),
            root: node(15),
            evaluation: name("postponed"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
        AnnotationEvaluationExpectedRow {
            id: node(16),
            root: node(15),
            evaluation: name("postponed"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
        AnnotationEvaluationExpectedRow {
            id: node(60),
            root: node(15),
            evaluation: name("postponed"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
        AnnotationEvaluationExpectedRow {
            id: node(19),
            root: node(19),
            evaluation: name("unevaluated"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
        AnnotationEvaluationExpectedRow {
            id: node(20),
            root: node(19),
            evaluation: name("unevaluated"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
        AnnotationEvaluationExpectedRow {
            id: node(21),
            root: node(19),
            evaluation: name("unevaluated"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
        AnnotationEvaluationExpectedRow {
            id: node(67),
            root: node(19),
            evaluation: name("unevaluated"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
        AnnotationEvaluationExpectedRow {
            id: node(68),
            root: node(19),
            evaluation: name("unevaluated"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
        AnnotationEvaluationExpectedRow {
            id: node(69),
            root: node(19),
            evaluation: name("unevaluated"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
        AnnotationEvaluationExpectedRow {
            id: node(70),
            root: node(19),
            evaluation: name("unevaluated"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
        AnnotationEvaluationExpectedRow {
            id: node(29),
            root: node(29),
            evaluation: name("postponed"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
        AnnotationEvaluationExpectedRow {
            id: node(94),
            root: node(29),
            evaluation: name("postponed"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
    ])
}
