// The rows `relations/annotation_evaluation/input.py` must give: under the annotations future every annotation is postponed (a
// parameter, a return, a class variable), a local variable's is never evaluated, and the string's own expression has an
// evaluation of its own and no row.

use ryo::languages::contract::{AnnotationEvaluationExpectedRow, Expected, expected_rows, file, name, node};

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(&[
        AnnotationEvaluationExpectedRow {
            id: node(28),
            root: node(28),
            evaluation: name("postponed"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
        AnnotationEvaluationExpectedRow {
            id: node(29),
            root: node(28),
            evaluation: name("postponed"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
        AnnotationEvaluationExpectedRow {
            id: node(38),
            root: node(38),
            evaluation: name("postponed"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
        AnnotationEvaluationExpectedRow {
            id: node(39),
            root: node(38),
            evaluation: name("postponed"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
        AnnotationEvaluationExpectedRow {
            id: node(44),
            root: node(44),
            evaluation: name("postponed"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
        AnnotationEvaluationExpectedRow {
            id: node(45),
            root: node(44),
            evaluation: name("postponed"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
        AnnotationEvaluationExpectedRow {
            id: node(46),
            root: node(44),
            evaluation: name("postponed"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
        AnnotationEvaluationExpectedRow {
            id: node(55),
            root: node(55),
            evaluation: name("unevaluated"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
        AnnotationEvaluationExpectedRow {
            id: node(56),
            root: node(55),
            evaluation: name("unevaluated"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
        AnnotationEvaluationExpectedRow {
            id: node(57),
            root: node(55),
            evaluation: name("unevaluated"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
        AnnotationEvaluationExpectedRow {
            id: node(58),
            root: node(55),
            evaluation: name("unevaluated"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
        AnnotationEvaluationExpectedRow {
            id: node(59),
            root: node(55),
            evaluation: name("unevaluated"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
        AnnotationEvaluationExpectedRow {
            id: node(60),
            root: node(55),
            evaluation: name("unevaluated"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
        AnnotationEvaluationExpectedRow {
            id: node(61),
            root: node(55),
            evaluation: name("unevaluated"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
        AnnotationEvaluationExpectedRow {
            id: node(92),
            root: node(92),
            evaluation: name("postponed"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
        AnnotationEvaluationExpectedRow {
            id: node(93),
            root: node(92),
            evaluation: name("postponed"),
            mode: name("lazy"),
            file: file("annotation_evaluation/input.py"),
        },
    ])
}
