//! The `node_fields` service: the field each child of a node fills, read from the typed node (the fields of `Python.asdl`, by
//! structure), in the order the children stand in the file. The engine's node table holds a field per child; the parser's walk
//! gives each child without one, so the walk asks here as it opens a node and hands each child its field as the child opens.

use crate::domain::expression_fields::service::{each, expression_child_fields, maybe, put};
use crate::domain::labels::types::Field;
use crate::domain::node::types::{ChildField, Placed};
use ruff_python_ast::{AnyNodeRef, Stmt};

/// Adds every statement of a block under one field.
fn block<'t>(out: &mut Vec<ChildField<'t>>, field: Field, statements: &'t [Stmt]) {
    for statement in statements {
        put(out, field, statement);
    }
}

/// Adds the children of `placed` to `out`, each with the field it fills, in the order they are written. A child the standard gives
/// no field is left out: the walk gives it none.
pub(crate) fn child_fields<'t>(placed: Placed<'t>, out: &mut Vec<ChildField<'t>>) {
    definition_fields(placed.node, out);
    statement_fields(placed.node, out);
    clause_fields(placed, out);
    expression_child_fields(placed.node, out);
}

/// The fields of a module, a function, a class and their parts.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only definitions and their parts are read here; every other kind is read by another function of the service"
)]
fn definition_fields<'t>(node: AnyNodeRef<'t>, out: &mut Vec<ChildField<'t>>) {
    match node {
        AnyNodeRef::ModModule(module) => block(out, Field::Body, &module.body),
        AnyNodeRef::ModExpression(module) => put(out, Field::Body, module.body.as_ref()),
        AnyNodeRef::StmtFunctionDef(function) => {
            for decorator in &function.decorator_list {
                put(out, Field::DecoratorList, decorator);
            }
            put(out, Field::Name, &function.name);
            if let Some(parameters) = &function.type_params {
                put(out, Field::TypeParams, parameters.as_ref());
            }
            put(out, Field::Args, function.parameters.as_ref());
            maybe(out, Field::Returns, function.returns.as_deref());
            block(out, Field::Body, &function.body);
        }
        AnyNodeRef::StmtClassDef(class) => {
            for decorator in &class.decorator_list {
                put(out, Field::DecoratorList, decorator);
            }
            put(out, Field::Name, &class.name);
            if let Some(parameters) = &class.type_params {
                put(out, Field::TypeParams, parameters.as_ref());
            }
            block(out, Field::Body, &class.body);
        }
        AnyNodeRef::ExprLambda(lambda) => {
            if let Some(parameters) = &lambda.parameters {
                put(out, Field::Args, parameters.as_ref());
            }
            put(out, Field::Body, lambda.body.as_ref());
        }
        AnyNodeRef::Parameters(parameters) => {
            for parameter in &parameters.posonlyargs {
                put(out, Field::Posonlyargs, parameter);
            }
            for parameter in &parameters.args {
                put(out, Field::Args, parameter);
            }
            if let Some(parameter) = &parameters.vararg {
                put(out, Field::Vararg, parameter.as_ref());
            }
            for parameter in &parameters.kwonlyargs {
                put(out, Field::Kwonlyargs, parameter);
            }
            if let Some(parameter) = &parameters.kwarg {
                put(out, Field::Kwarg, parameter.as_ref());
            }
        }
        AnyNodeRef::Parameter(parameter) => {
            put(out, Field::Arg, &parameter.name);
            maybe(out, Field::Annotation, parameter.annotation.as_deref());
        }
        AnyNodeRef::Decorator(decorator) => put(out, Field::None, &decorator.expression),
        AnyNodeRef::StmtTypeAlias(alias) => {
            put(out, Field::Name, alias.name.as_ref());
            if let Some(parameters) = &alias.type_params {
                put(out, Field::TypeParams, parameters.as_ref());
            }
            put(out, Field::Value, alias.value.as_ref());
        }
        AnyNodeRef::TypeParamTypeVar(parameter) => {
            put(out, Field::Name, &parameter.name);
            maybe(out, Field::Bound, parameter.bound.as_deref());
            maybe(out, Field::DefaultValue, parameter.default.as_deref());
        }
        AnyNodeRef::TypeParamTypeVarTuple(parameter) => {
            put(out, Field::Name, &parameter.name);
            maybe(out, Field::DefaultValue, parameter.default.as_deref());
        }
        AnyNodeRef::TypeParamParamSpec(parameter) => {
            put(out, Field::Name, &parameter.name);
            maybe(out, Field::DefaultValue, parameter.default.as_deref());
        }
        _ => {}
    }
}

/// The fields of a statement that is no definition.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only statements are read here; every other kind is read by another function of the service"
)]
#[expect(
    clippy::too_many_lines,
    reason = "one arm for each statement of the standard that holds a child: the length is the grammar's"
)]
fn statement_fields<'t>(node: AnyNodeRef<'t>, out: &mut Vec<ChildField<'t>>) {
    match node {
        AnyNodeRef::StmtReturn(statement) => maybe(out, Field::Value, statement.value.as_deref()),
        AnyNodeRef::StmtDelete(statement) => each(out, Field::Targets, &statement.targets),
        AnyNodeRef::StmtAssign(statement) => {
            each(out, Field::Targets, &statement.targets);
            put(out, Field::Value, statement.value.as_ref());
        }
        AnyNodeRef::StmtAugAssign(statement) => {
            put(out, Field::Target, statement.target.as_ref());
            put(out, Field::Value, statement.value.as_ref());
        }
        AnyNodeRef::StmtAnnAssign(statement) => {
            put(out, Field::Target, statement.target.as_ref());
            put(out, Field::Annotation, statement.annotation.as_ref());
            maybe(out, Field::Value, statement.value.as_deref());
        }
        AnyNodeRef::StmtFor(statement) => {
            put(out, Field::Target, statement.target.as_ref());
            put(out, Field::Iter, statement.iter.as_ref());
            block(out, Field::Body, &statement.body);
            block(out, Field::Orelse, &statement.orelse);
        }
        AnyNodeRef::StmtWhile(statement) => {
            put(out, Field::Test, statement.test.as_ref());
            block(out, Field::Body, &statement.body);
            block(out, Field::Orelse, &statement.orelse);
        }
        AnyNodeRef::StmtIf(statement) => {
            put(out, Field::Test, statement.test.as_ref());
            block(out, Field::Body, &statement.body);
            for clause in &statement.elif_else_clauses {
                put(out, Field::Orelse, clause);
            }
        }
        AnyNodeRef::ElifElseClause(clause) => {
            maybe(out, Field::Test, clause.test.as_ref());
            block(out, Field::Body, &clause.body);
        }
        AnyNodeRef::StmtWith(statement) => {
            for item in &statement.items {
                put(out, Field::Items, item);
            }
            block(out, Field::Body, &statement.body);
        }
        AnyNodeRef::WithItem(item) => {
            put(out, Field::ContextExpr, &item.context_expr);
            maybe(out, Field::OptionalVars, item.optional_vars.as_deref());
        }
        AnyNodeRef::StmtRaise(statement) => {
            maybe(out, Field::Exc, statement.exc.as_deref());
            maybe(out, Field::Cause, statement.cause.as_deref());
        }
        AnyNodeRef::StmtTry(statement) => {
            block(out, Field::Body, &statement.body);
            for handler in &statement.handlers {
                put(out, Field::Handlers, handler);
            }
            block(out, Field::Orelse, &statement.orelse);
            block(out, Field::Finalbody, &statement.finalbody);
        }
        AnyNodeRef::ExceptHandlerExceptHandler(handler) => {
            maybe(out, Field::Type, handler.type_.as_deref());
            if let Some(name) = &handler.name {
                put(out, Field::Name, name);
            }
            block(out, Field::Body, &handler.body);
        }
        AnyNodeRef::StmtAssert(statement) => {
            put(out, Field::Test, statement.test.as_ref());
            maybe(out, Field::Msg, statement.msg.as_deref());
        }
        AnyNodeRef::StmtImport(statement) => {
            for alias in &statement.names {
                put(out, Field::Names, alias);
            }
        }
        AnyNodeRef::StmtImportFrom(statement) => {
            if let Some(module) = &statement.module {
                put(out, Field::Module, module);
            }
            for alias in &statement.names {
                put(out, Field::Names, alias);
            }
        }
        AnyNodeRef::Alias(alias) => {
            put(out, Field::Name, &alias.name);
            if let Some(name) = &alias.asname {
                put(out, Field::Asname, name);
            }
        }
        AnyNodeRef::StmtGlobal(statement) => {
            for name in &statement.names {
                put(out, Field::Names, name);
            }
        }
        AnyNodeRef::StmtNonlocal(statement) => {
            for name in &statement.names {
                put(out, Field::Names, name);
            }
        }
        AnyNodeRef::StmtExpr(statement) => put(out, Field::Value, statement.value.as_ref()),
        AnyNodeRef::StmtMatch(statement) => {
            put(out, Field::Subject, statement.subject.as_ref());
            for case in &statement.cases {
                put(out, Field::Cases, case);
            }
        }
        AnyNodeRef::MatchCase(case) => {
            put(out, Field::Pattern, &case.pattern);
            maybe(out, Field::Guard, case.guard.as_deref());
            block(out, Field::Body, &case.body);
        }
        _ => {}
    }
}

/// The fields of the two nodes whose children's fields depend on where the node stands: a call's arguments are a class's bases
/// under a class, and a parameter with a default stands for its parameter, in the list that holds it.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only an argument list and a parameter with its default are read here; every other kind is read by another function"
)]
fn clause_fields<'t>(placed: Placed<'t>, out: &mut Vec<ChildField<'t>>) {
    match placed.node {
        AnyNodeRef::Arguments(arguments) => {
            let positional = if matches!(placed.parent, Some(AnyNodeRef::StmtClassDef(_))) {
                Field::Bases
            } else {
                Field::Args
            };
            for argument in arguments.iter_source_order() {
                match argument {
                    ruff_python_ast::ArgOrKeyword::Arg(expression) => {
                        put(out, positional, expression);
                    }
                    ruff_python_ast::ArgOrKeyword::Keyword(keyword) => {
                        put(out, Field::Keywords, keyword);
                    }
                }
            }
        }
        AnyNodeRef::ParameterWithDefault(parameter) => {
            put(out, placed.field, &parameter.parameter);
            let defaults = if placed.field == Field::Kwonlyargs {
                Field::KwDefaults
            } else {
                Field::Defaults
            };
            maybe(out, defaults, parameter.default.as_deref());
        }
        _ => {}
    }
}
