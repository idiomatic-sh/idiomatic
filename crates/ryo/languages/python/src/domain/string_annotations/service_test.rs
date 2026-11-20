//! The `string_annotations` service's cases: which strings of a source stand where the language reads a type expression.

use super::service::{form_imports, type_position};
use super::types::{FormImports, FormSource};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lexical_facts::types::QualifiedPath;
use crate::domain::model::view::types::{Built, SemanticModel};
use crate::domain::node::service::{typed_nodes, written};
use crate::domain::node::types::{AnnotationSeed, StringSite};
use crate::domain::semantic_syntax::types::TypeContext;
use crate::fixtures::syntax_files::with_file;
use ruff_python_ast::AnyNodeRef;
use ryo::languages::Position;

/// A source, and where its strings stand by their syntax alone.
type Input = (&'static str, AnnotationSeed);
/// The strings of the source the language reads as type expressions, as they are written, joined by `|`.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_string_in_an_annotation_is_a_type_and_an_operand_of_literal_is_not"),
        word: Word::Succeeds,
        input: || {
            (
                "from typing import Literal, List\nx: 'A'\ny: List['B']\nz: Literal['c']\n",
                AnnotationSeed::Annotation,
            )
        },
        expect: Expect::Value("'A'|'B'|model built: true"),
    },
    Case {
        name: CaseName("a_string_argument_is_a_type_only_where_a_typing_form_reads_it_as_one"),
        word: Word::Succeeds,
        input: || {
            (
                "import typing\nfrom typing import cast as c\nc('A', 'b')\ntyping.TypeVar('T', 'C', bound='D')\nprint('e')\n",
                AnnotationSeed::Candidate,
            )
        },
        expect: Expect::Value("'A'|'C'|'D'|model built: true"),
    },
    Case {
        name: CaseName("an_operand_of_a_builtin_generic_class_is_a_type_wherever_it_stands"),
        word: Word::Succeeds,
        input: || {
            (
                "d = list['E']\nlist = g\nh = list['I']\n",
                AnnotationSeed::Candidate,
            )
        },
        expect: Expect::Value("'E'|model built: true"),
    },
    Case {
        name: CaseName("a_name_no_import_brings_is_no_form_and_the_model_is_not_built"),
        word: Word::Succeeds,
        input: || ("cast('A', b)\nf = g['H']\n", AnnotationSeed::Candidate),
        expect: Expect::Value("model built: false"),
    },
];

fn subject(asked: Input) -> Result<Value, Failure> {
    let mut shown: Vec<String> = Vec::new();
    with_file(asked.0, &mut |file| {
        let aliases: Vec<Position> = typed_nodes(file)
            .filter(|each| matches!(each.node, AnyNodeRef::Alias(_)))
            .map(|each| each.id)
            .collect();
        let mut imports = FormImports::default();
        form_imports(file.tree, &aliases, &mut imports);
        let (mut model, mut built) = (SemanticModel::default(), Built::default());
        let mut path = QualifiedPath::default();
        let mut source = FormSource {
            tree: file.tree,
            imports: &imports,
            model: &mut model,
            built: &mut built,
            path: &mut path,
        };
        for each in typed_nodes(file) {
            if matches!(each.node, AnyNodeRef::ExprStringLiteral(_))
                && type_position(
                    &mut source,
                    StringSite {
                        at: each.id,
                        seed: asked.1,
                    },
                ) == TypeContext::Type
            {
                shown.push(written(file, each.id).0.to_owned());
            }
        }
        shown.push(format!("model built: {}", built != Built::default()));
    });
    Ok(Box::leak(shown.join("|").into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
