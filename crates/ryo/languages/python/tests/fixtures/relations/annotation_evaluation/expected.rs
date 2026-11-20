// Annotation evaluation timing, one input per clause group; rows recorded from a run and checked against clauses A1-A3.

use ryo::languages::contract::{AnnotationEvaluationExpectedRow, Expected, expected_rows, file, name, node};

/// Independently specified complete rows for these inputs.
pub(super) fn expected() -> Expected {
    expected_rows(&[
        AnnotationEvaluationExpectedRow { id: node(6), root: node(6), evaluation: name("deferred"), mode: name("lazy"), file: file("annotation_evaluation/bodies.py") },
        AnnotationEvaluationExpectedRow { id: node(7), root: node(6), evaluation: name("deferred"), mode: name("lazy"), file: file("annotation_evaluation/bodies.py") },
        AnnotationEvaluationExpectedRow { id: node(8), root: node(6), evaluation: name("deferred"), mode: name("lazy"), file: file("annotation_evaluation/bodies.py") },
        AnnotationEvaluationExpectedRow { id: node(9), root: node(6), evaluation: name("deferred"), mode: name("lazy"), file: file("annotation_evaluation/bodies.py") },
        AnnotationEvaluationExpectedRow { id: node(23), root: node(23), evaluation: name("deferred"), mode: name("lazy"), file: file("annotation_evaluation/bodies.py") },
        AnnotationEvaluationExpectedRow { id: node(24), root: node(23), evaluation: name("deferred"), mode: name("lazy"), file: file("annotation_evaluation/bodies.py") },
        AnnotationEvaluationExpectedRow { id: node(27), root: node(23), evaluation: name("deferred"), mode: name("lazy"), file: file("annotation_evaluation/bodies.py") },
        AnnotationEvaluationExpectedRow { id: node(28), root: node(23), evaluation: name("deferred"), mode: name("lazy"), file: file("annotation_evaluation/bodies.py") },
        AnnotationEvaluationExpectedRow { id: node(36), root: node(23), evaluation: name("deferred"), mode: name("lazy"), file: file("annotation_evaluation/bodies.py") },
        AnnotationEvaluationExpectedRow { id: node(37), root: node(23), evaluation: name("deferred"), mode: name("lazy"), file: file("annotation_evaluation/bodies.py") },
        AnnotationEvaluationExpectedRow { id: node(38), root: node(23), evaluation: name("deferred"), mode: name("lazy"), file: file("annotation_evaluation/bodies.py") },
        AnnotationEvaluationExpectedRow { id: node(26), root: node(26), evaluation: name("postponed"), mode: name("lazy"), file: file("annotation_evaluation/future.py") },
        AnnotationEvaluationExpectedRow { id: node(27), root: node(26), evaluation: name("postponed"), mode: name("lazy"), file: file("annotation_evaluation/future.py") },
        AnnotationEvaluationExpectedRow { id: node(28), root: node(26), evaluation: name("postponed"), mode: name("lazy"), file: file("annotation_evaluation/future.py") },
        AnnotationEvaluationExpectedRow { id: node(29), root: node(26), evaluation: name("postponed"), mode: name("lazy"), file: file("annotation_evaluation/future.py") },
        AnnotationEvaluationExpectedRow { id: node(30), root: node(26), evaluation: name("postponed"), mode: name("lazy"), file: file("annotation_evaluation/future.py") },
        AnnotationEvaluationExpectedRow { id: node(31), root: node(26), evaluation: name("postponed"), mode: name("lazy"), file: file("annotation_evaluation/future.py") },
        AnnotationEvaluationExpectedRow { id: node(32), root: node(26), evaluation: name("postponed"), mode: name("lazy"), file: file("annotation_evaluation/future.py") },
        AnnotationEvaluationExpectedRow { id: node(33), root: node(26), evaluation: name("postponed"), mode: name("lazy"), file: file("annotation_evaluation/future.py") },
        AnnotationEvaluationExpectedRow { id: node(34), root: node(26), evaluation: name("postponed"), mode: name("lazy"), file: file("annotation_evaluation/future.py") },
        AnnotationEvaluationExpectedRow { id: node(35), root: node(26), evaluation: name("postponed"), mode: name("lazy"), file: file("annotation_evaluation/future.py") },
        AnnotationEvaluationExpectedRow { id: node(36), root: node(26), evaluation: name("postponed"), mode: name("lazy"), file: file("annotation_evaluation/future.py") },
        AnnotationEvaluationExpectedRow { id: node(37), root: node(26), evaluation: name("postponed"), mode: name("lazy"), file: file("annotation_evaluation/future.py") },
        AnnotationEvaluationExpectedRow { id: node(47), root: node(47), evaluation: name("unevaluated"), mode: name("lazy"), file: file("annotation_evaluation/future.py") },
        AnnotationEvaluationExpectedRow { id: node(48), root: node(47), evaluation: name("unevaluated"), mode: name("lazy"), file: file("annotation_evaluation/future.py") },
        AnnotationEvaluationExpectedRow { id: node(14), root: node(14), evaluation: name("deferred"), mode: name("lazy"), file: file("annotation_evaluation/input.py") },
        AnnotationEvaluationExpectedRow { id: node(15), root: node(14), evaluation: name("deferred"), mode: name("lazy"), file: file("annotation_evaluation/input.py") },
        AnnotationEvaluationExpectedRow { id: node(20), root: node(20), evaluation: name("deferred"), mode: name("lazy"), file: file("annotation_evaluation/input.py") },
        AnnotationEvaluationExpectedRow { id: node(21), root: node(20), evaluation: name("deferred"), mode: name("lazy"), file: file("annotation_evaluation/input.py") },
        AnnotationEvaluationExpectedRow { id: node(30), root: node(30), evaluation: name("unevaluated"), mode: name("lazy"), file: file("annotation_evaluation/input.py") },
        AnnotationEvaluationExpectedRow { id: node(31), root: node(30), evaluation: name("unevaluated"), mode: name("lazy"), file: file("annotation_evaluation/input.py") },
    ])
}
