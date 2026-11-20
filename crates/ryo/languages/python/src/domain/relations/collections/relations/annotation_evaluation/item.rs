//! Emit annotation evaluation timing once from the module root.
use crate::domain::{
    annotation_evaluation::{
        service::evaluated,
        types::{Evaluation, Mode},
    },
    model::types::Declarations,
    nodes::types::Node,
    syntax::{service::kind_bits, types::Kind},
};
use ryo::languages;
/// For each element directly inside an annotation, the annotation and when the language evaluates it.
pub(crate) struct AnnotationEvaluationItem;
/// The query word of a timing.
const fn evaluation_word(evaluation: Evaluation) -> &'static str {
    match evaluation {
        Evaluation::Eager => "eager",
        Evaluation::Postponed => "postponed",
        Evaluation::Deferred => "deferred",
        Evaluation::Unevaluated => "unevaluated",
    }
}
/// The query word of a mode.
const fn mode_word(mode: Mode) -> &'static str {
    match mode {
        Mode::Lazy => "lazy",
        Mode::Eager => "eager",
    }
}
ryo::reads! {
 AnnotationEvaluationItem => languages::AnnotationEvaluation, model(Declarations) |tree, model, at, symbols| {
  kinds:[Kind::Node(Node::Module)] as kind_bits,
  when evaluated(tree, model) {
   None=>{partial super::super::compiler_feature::constants::INCOMPLETE => languages::Missing::Any;}
   Some(scratch)=>{
    each (scratch.members.iter()) as answer {
     given evaluation = languages::intern_derived(symbols, tree.source, languages::NameText(evaluation_word(answer.evaluation)));
     given mode = languages::intern_derived(symbols, tree.source, languages::NameText(mode_word(answer.mode)));
row { id:node(answer.id),root:node(answer.root),evaluation:value(evaluation),mode:value(mode),file:file, }
    }
   }
  }
 }
}
