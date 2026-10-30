//! The `refinements` service: free functions named by verb. What Rust declares keeps its own
//! distinction in a row beside the shared kind, and this says which word it is.

use super::collections::refinements::associated_function::item::AssociatedFunctionItem;
use super::collections::refinements::const_::item::ConstItem;
use super::collections::refinements::const_parameter::item::ConstParameterItem;
use super::collections::refinements::enum_::item::EnumItem;
use super::collections::refinements::extern_crate::item::ExternCrateItem;
use super::collections::refinements::field::item::FieldItem;
use super::collections::refinements::function::item::FunctionItem;
use super::collections::refinements::impl_::item::ImplItem;
use super::collections::refinements::interface::RefinementsItem as _;
use super::collections::refinements::label::item::LabelItem;
use super::collections::refinements::lifetime_parameter::item::LifetimeParameterItem;
use super::collections::refinements::macro_::item::MacroItem;
use super::collections::refinements::macro_rules::item::MacroRulesItem;
use super::collections::refinements::method::item::MethodItem;
use super::collections::refinements::module::item::ModuleItem;
use super::collections::refinements::positional::item::PositionalItem;
use super::collections::refinements::receiver::item::ReceiverItem;
use super::collections::refinements::static_::item::StaticItem;
use super::collections::refinements::struct_::item::StructItem;
use super::collections::refinements::trait_::item::TraitItem;
use super::collections::refinements::tuple_field::item::TupleFieldItem;
use super::collections::refinements::type_alias::item::TypeAliasItem;
use super::collections::refinements::type_parameter::item::TypeParameterItem;
use super::collections::refinements::union::item::UnionItem;
use super::collections::refinements::use_::item::UseItem;
use super::collections::refinements::variable::item::VariableItem;
use super::collections::refinements::variadic::item::VariadicItem;
use super::collections::refinements::variant::item::VariantItem;
use crate::domain::refinements::types::Declared;
use ryo::languages;

/// What a declaration is in Rust's own words: text that leaves the engine.
#[must_use]
pub(crate) fn refinement_name(declared: Declared) -> languages::NameText<'static> {
    languages::NameText(match declared {
        Declared::Function => FunctionItem::TEXT,
        Declared::Struct => StructItem::TEXT,
        Declared::Enum => EnumItem::TEXT,
        Declared::Trait => TraitItem::TEXT,
        Declared::Variable => VariableItem::TEXT,
        Declared::Const => ConstItem::TEXT,
        Declared::Static => StaticItem::TEXT,
        Declared::Module => ModuleItem::TEXT,
        Declared::TypeAlias => TypeAliasItem::TEXT,
        Declared::Union => UnionItem::TEXT,
        Declared::MacroRules => MacroRulesItem::TEXT,
        Declared::Macro => MacroItem::TEXT,
        Declared::Method => MethodItem::TEXT,
        Declared::AssociatedFunction => AssociatedFunctionItem::TEXT,
        Declared::Variant => VariantItem::TEXT,
        Declared::Field => FieldItem::TEXT,
        Declared::TupleField => TupleFieldItem::TEXT,
        Declared::TypeParameter => TypeParameterItem::TEXT,
        Declared::LifetimeParameter => LifetimeParameterItem::TEXT,
        Declared::ConstParameter => ConstParameterItem::TEXT,
        Declared::Use => UseItem::TEXT,
        Declared::ExternCrate => ExternCrateItem::TEXT,
        Declared::Implementation => ImplItem::TEXT,
        Declared::Label => LabelItem::TEXT,
    })
}

/// What a parameter is in Rust's own words: the receiver of a method.
#[must_use]
pub(crate) const fn receiver_name() -> languages::NameText<'static> {
    languages::NameText(ReceiverItem::TEXT)
}

/// What a parameter is in Rust's own words: one passed by position.
#[must_use]
pub(crate) const fn positional_name() -> languages::NameText<'static> {
    languages::NameText(PositionalItem::TEXT)
}

/// What a parameter is in Rust's own words: the C-variadic `...` that takes the arguments left over.
#[must_use]
pub(crate) const fn variadic_name() -> languages::NameText<'static> {
    languages::NameText(VariadicItem::TEXT)
}
