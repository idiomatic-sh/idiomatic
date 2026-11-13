//! The request `compile_rules` takes: the rules, the relations the host declares and the kinds of the linked languages.

/// What a caller asks to have compiled.
#[derive(Clone, Copy, Debug)]
pub struct CompileRequest<'a> {
    /// The rules, in their order.
    pub rules: &'a [ryo_values::domain::rule_requests::types::RuleSpec],
    /// The relations the host declares.
    pub hosted: &'a [ryo_values::domain::hosted::types::HostRelation],
    /// The kinds of every linked language, which a rule's kind words must name.
    pub vocabularies: &'a ryo_values::domain::descriptions::types::KindVocabularies,
}
