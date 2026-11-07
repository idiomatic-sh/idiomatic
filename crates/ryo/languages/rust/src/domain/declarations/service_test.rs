//! The `declarations` service's cases, on values alone: the kind each thing Rust declares is answered as, in the closed set every language
//! answers in.

use super::service::declaration_kind;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::refinements::types::Declared;
use ryo::languages;

/// What Rust declares.
type Input = Declared;
/// The kind it is answered as.
type Value = languages::DeclarationKind;
type Failure = ();

/// The rows of this part's cases: one for each thing Rust declares.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("function_is_declared_as_a_function"),
        word: Word::Succeeds,
        input: || Declared::Function,
        expect: Expect::Value(languages::DeclarationKind::Function),
    },
    Case {
        name: CaseName("struct_is_declared_as_a_struct"),
        word: Word::Succeeds,
        input: || Declared::Struct,
        expect: Expect::Value(languages::DeclarationKind::Struct),
    },
    Case {
        name: CaseName("union_is_declared_as_a_struct"),
        word: Word::Succeeds,
        input: || Declared::Union,
        expect: Expect::Value(languages::DeclarationKind::Struct),
    },
    Case {
        name: CaseName("enum_is_declared_as_a_enum"),
        word: Word::Succeeds,
        input: || Declared::Enum,
        expect: Expect::Value(languages::DeclarationKind::Enum),
    },
    Case {
        name: CaseName("trait_is_declared_as_a_trait"),
        word: Word::Succeeds,
        input: || Declared::Trait,
        expect: Expect::Value(languages::DeclarationKind::Trait),
    },
    Case {
        name: CaseName("variable_is_declared_as_a_variable"),
        word: Word::Succeeds,
        input: || Declared::Variable,
        expect: Expect::Value(languages::DeclarationKind::Variable),
    },
    Case {
        name: CaseName("static_is_declared_as_a_variable"),
        word: Word::Succeeds,
        input: || Declared::Static,
        expect: Expect::Value(languages::DeclarationKind::Variable),
    },
    Case {
        name: CaseName("const_is_declared_as_a_constant"),
        word: Word::Succeeds,
        input: || Declared::Const,
        expect: Expect::Value(languages::DeclarationKind::Constant),
    },
    Case {
        name: CaseName("module_is_declared_as_a_namespace"),
        word: Word::Succeeds,
        input: || Declared::Module,
        expect: Expect::Value(languages::DeclarationKind::Namespace),
    },
    Case {
        name: CaseName("type_alias_is_declared_as_a_type_alias"),
        word: Word::Succeeds,
        input: || Declared::TypeAlias,
        expect: Expect::Value(languages::DeclarationKind::TypeAlias),
    },
    Case {
        name: CaseName("macro_rules_is_declared_as_a_macro"),
        word: Word::Succeeds,
        input: || Declared::MacroRules,
        expect: Expect::Value(languages::DeclarationKind::Macro),
    },
    Case {
        name: CaseName("macro_is_declared_as_a_macro"),
        word: Word::Succeeds,
        input: || Declared::Macro,
        expect: Expect::Value(languages::DeclarationKind::Macro),
    },
    Case {
        name: CaseName("method_is_declared_as_a_method"),
        word: Word::Succeeds,
        input: || Declared::Method,
        expect: Expect::Value(languages::DeclarationKind::Method),
    },
    Case {
        name: CaseName("associated_function_is_declared_as_a_method"),
        word: Word::Succeeds,
        input: || Declared::AssociatedFunction,
        expect: Expect::Value(languages::DeclarationKind::Method),
    },
    Case {
        name: CaseName("variant_is_declared_as_a_enum_member"),
        word: Word::Succeeds,
        input: || Declared::Variant,
        expect: Expect::Value(languages::DeclarationKind::EnumMember),
    },
    Case {
        name: CaseName("field_is_declared_as_a_property"),
        word: Word::Succeeds,
        input: || Declared::Field,
        expect: Expect::Value(languages::DeclarationKind::Property),
    },
    Case {
        name: CaseName("tuple_field_is_declared_as_a_property"),
        word: Word::Succeeds,
        input: || Declared::TupleField,
        expect: Expect::Value(languages::DeclarationKind::Property),
    },
    Case {
        name: CaseName("type_parameter_is_declared_as_a_type_parameter"),
        word: Word::Succeeds,
        input: || Declared::TypeParameter,
        expect: Expect::Value(languages::DeclarationKind::TypeParameter),
    },
    Case {
        name: CaseName("lifetime_parameter_is_declared_as_a_type_parameter"),
        word: Word::Succeeds,
        input: || Declared::LifetimeParameter,
        expect: Expect::Value(languages::DeclarationKind::TypeParameter),
    },
    Case {
        name: CaseName("const_parameter_is_declared_as_a_type_parameter"),
        word: Word::Succeeds,
        input: || Declared::ConstParameter,
        expect: Expect::Value(languages::DeclarationKind::TypeParameter),
    },
    Case {
        name: CaseName("use_is_declared_as_a_import"),
        word: Word::Succeeds,
        input: || Declared::Use,
        expect: Expect::Value(languages::DeclarationKind::Import),
    },
    Case {
        name: CaseName("extern_crate_is_declared_as_a_import"),
        word: Word::Succeeds,
        input: || Declared::ExternCrate,
        expect: Expect::Value(languages::DeclarationKind::Import),
    },
];

fn subject(declared: Input) -> Value {
    declaration_kind(declared)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
