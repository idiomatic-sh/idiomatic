//! The `expansion` service: a pack's programs as one plan.

use core::hash::BuildHasher as _;
use super::types::{Expansion, FragmentInstance, ProgramScope};
use crate::domain::finish_plans::arguments::literals::types::{
    ArgumentValue as StepArgument, StepArgumentName, UnlistedArgument,
};
use crate::domain::finish_plans::arguments::types::StepNumber;
use crate::domain::finish_plans::constants::{ARG_FROM, OP_ROWS, RELATION_ARGUMENTS};
use crate::domain::finish_plans::expressions::types::{ConstantEntry, PlanConstant};
use crate::domain::finish_plans::types::{
    CountMark, EmitSubject, FinishPlan, FinishStep, PlanEmit, PlannedRule, ProbeMark, RuleEmit,
    SelectItem, SelectSource, SelectStep, SelectedColumns, UnplannedReason, UnplannedRule,
};
use crate::domain::plan_lowering::errors::PlanLoweringError;
use crate::domain::plan_lowering::filters::service::{alias, column, constant, function, select_condition};
use crate::domain::plan_lowering::operators::service::operator_step;
use crate::domain::plan_lowering::types::{PackPrograms, RuleProgram, WrittenText};
use ryo_values::domain::descriptions::types::RelationName;
use ryo_values::domain::program::types::{FragmentProgram, ImportDeclaration, LetStep, WrittenProgram};
use ryo_values::domain::query_syntax::types::{
    Argument, ArgumentValue, Call, Columns, Expression, Joining, QuerySource, Renaming, Select,
    SelectedValue, SourceReference,
};
use ryo_values::domain::query_tokens::types::QuerySourceId;

/// The plan of every rule of the pack, in rule order, with no planner mark.
pub(crate) fn expanded(pack: &PackPrograms) -> FinishPlan {
    let mut expansion = Expansion {
        steps: Vec::new(),
        numbers: hashbrown::HashTable::new(),
        hasher: hashbrown::DefaultHashBuilder::default(),
        fragments: &pack.fragments.0,
        instances: hashbrown::HashMap::new(),
    };
    let mut plan = FinishPlan {
        steps: Vec::new(),
        rules: Vec::new(),
        unplanned: Vec::new(),
    };
    for rule in &pack.rules {
        let lowered = match &rule.program {
            RuleProgram::Parsed(program) => rule_emit(&mut expansion, program),
            RuleProgram::Unread(why) => Err(PlanLoweringError::Unread(WrittenText(why.0.clone()))),
        };
        match lowered {
            Ok(emit) => plan.rules.push(PlannedRule {
                rule: rule.rule.clone(),
                emit,
            }),
            Err(refusal) => plan.unplanned.push(UnplannedRule {
                rule: rule.rule.clone(),
                reason: UnplannedReason(refusal.to_string().into()),
            }),
        }
    }
    plan.steps = expansion.steps;
    plan
}

/// A rule's program lowered: the step it emits.
fn rule_emit<'p>(
    expansion: &mut Expansion<'p>,
    program: &'p WrittenProgram,
) -> Result<RuleEmit, PlanLoweringError> {
    if let Some(parameter) = program.parameters.first() {
        return Err(PlanLoweringError::NotGiven(WrittenText(
            parameter.parameter.0.clone(),
        )));
    }
    let mut scope = ProgramScope {
        imports: imported(&program.imports),
        ..ProgramScope::default()
    };
    lets(expansion, &mut scope, &program.steps)?;
    let step = named(&scope, &program.emit.step.0.0)?;
    Ok(RuleEmit::Written(PlanEmit {
        step,
        subject: EmitSubject(program.emit.subject.0.clone()),
        key: program.emit.key.clone(),
    }))
}

/// Each import's name and the fragment it names: the last part of its path.
fn imported(imports: &[ImportDeclaration]) -> hashbrown::HashMap<&str, &str> {
    imports
        .iter()
        .map(|import| {
            let path = &*import.module.0;
            (&*import.alias.0, path.rsplit('/').next().unwrap_or(path))
        })
        .collect()
}

/// Lowers each `let` in order, naming its step.
fn lets<'p>(
    expansion: &mut Expansion<'p>,
    scope: &mut ProgramScope<'p>,
    steps: &'p [LetStep],
) -> Result<(), PlanLoweringError> {
    for step in steps {
        let number = expression(expansion, scope, &step.expression)?;
        let _earlier = scope.steps.insert(&*step.name.0.0, number);
    }
    Ok(())
}

/// The step a name names.
fn named(scope: &ProgramScope<'_>, name: &str) -> Result<StepNumber, PlanLoweringError> {
    scope
        .steps
        .get(name)
        .copied()
        .ok_or_else(|| PlanLoweringError::UnknownStep(WrittenText(name.into())))
}

/// The step of a relation of the catalog.
fn base(expansion: &mut Expansion<'_>, relation: &str) -> StepNumber {
    numbered(expansion, FinishStep::Base(RelationName(relation.into())))
}

/// The number of a step: an equal step's when one was made, else the next. A step that holds no map is its own identity
/// and is compared as it is; only a step that holds one is copied into its identity.
fn numbered(expansion: &mut Expansion<'_>, step: FinishStep) -> StepNumber {
    let Expansion { steps, numbers, hasher, .. } = expansion;
    let mapped = holds_map(&step);
    let identity_form = mapped.then(|| identity(&step));
    let key = identity_form.as_ref().unwrap_or(&step);
    let hash = hasher.hash_one(key);
    let found = numbers.find(hash, |number| {
        steps.get(number.0).is_some_and(|made| if mapped { identity(made) == *key } else { made == key })
    });
    if let Some(number) = found {
        return *number;
    }
    let number = StepNumber(steps.len());
    let _slot = numbers.insert_unique(hash, number, |earlier| {
        steps.get(earlier.0).map_or(0, |made| {
            if holds_map(made) { hasher.hash_one(identity(made)) } else { hasher.hash_one(made) }
        })
    });
    steps.push(step);
    number
}

/// Whether a step holds a map, whose entries its identity orders.
fn holds_map(step: &FinishStep) -> bool {
    matches!(step, FinishStep::Extend(_) | FinishStep::AsOf(_) | FinishStep::OrderedMatch(_) | FinishStep::Unlisted(_))
}

/// A step's identity: the step with the entries of every map it holds in key order, so that two steps differing only
/// in the order a map was written are one.
fn identity(step: &FinishStep) -> FinishStep {
    let mut identity = step.clone();
    match &mut identity {
        FinishStep::Extend(extend) => {
            extend.values.sort_by(|a, b| a.name.cmp(&b.name));
            for value in &mut extend.values {
                sorted_constant(&mut value.value);
            }
        }
        FinishStep::AsOf(as_of) => as_of.output.sort_by(|a, b| a.name.cmp(&b.name)),
        FinishStep::OrderedMatch(matched) => matched.output.sort_by(|a, b| a.name.cmp(&b.name)),
        FinishStep::Unlisted(unlisted) => {
            unlisted.arguments.sort_by(|a, b| a.name.cmp(&b.name));
            for argument in &mut unlisted.arguments {
                if let StepArgument::Constant(constant) = &mut argument.value {
                    sorted_constant(constant);
                }
            }
        }
        FinishStep::Base(_)
        | FinishStep::Select(_)
        | FinishStep::Union(_)
        | FinishStep::Antijoin(_)
        | FinishStep::Axis(_)
        | FinishStep::Closure(_)
        | FinishStep::Follow(_)
        | FinishStep::OrderedDfs(_)
        | FinishStep::GroupCount(_)
        | FinishStep::GroupMin(_)
        | FinishStep::GroupMax(_)
        | FinishStep::GroupOrdinal(_)
        | FinishStep::Project(_)
        | FinishStep::SpanBounds(_)
        | FinishStep::SpanBetween(_)
        | FinishStep::Replace(_)
        | FinishStep::SourceMatches(_)
        | FinishStep::Capture(_)
        | FinishStep::FitsRendered(_)
        | FinishStep::RenderSyntax(_)
        | FinishStep::Rows(_)
        | FinishStep::Prefixes(_)
        | FinishStep::SubtreeClass(_) => {}
    }
    identity
}

/// A constant with every map's entries in key order.
fn sorted_constant(constant: &mut PlanConstant) {
    match constant {
        PlanConstant::List(items) => items.iter_mut().for_each(sorted_constant),
        PlanConstant::Map(entries) => {
            entries.sort_by(|a: &ConstantEntry, b: &ConstantEntry| a.key.cmp(&b.key));
            for entry in entries {
                sorted_constant(&mut entry.value);
            }
        }
        PlanConstant::Text(_) | PlanConstant::Number(_) | PlanConstant::Absent => {}
    }
}

/// The step of a `let`'s expression.
fn expression<'p>(
    expansion: &mut Expansion<'p>,
    scope: &ProgramScope<'p>,
    written: &'p Expression,
) -> Result<StepNumber, PlanLoweringError> {
    match written {
        Expression::Select(select) => selected(expansion, scope, select),
        Expression::Call(call) => called(expansion, scope, call),
    }
}

/// The step a source reads: a relation of the catalog, or a step the program named.
fn source_step(
    expansion: &mut Expansion<'_>,
    scope: &ProgramScope<'_>,
    source: &QuerySource,
) -> Result<SelectSource, PlanLoweringError> {
    let step = match &source.reference {
        SourceReference::Relation(relation) => base(expansion, &relation.0),
        SourceReference::Step(name) => named(scope, &name.0.0)?,
    };
    Ok(SelectSource {
        step,
        alias: alias(&source.alias),
    })
}

/// A select's step.
fn selected(
    expansion: &mut Expansion<'_>,
    scope: &ProgramScope<'_>,
    select: &Select,
) -> Result<StepNumber, PlanLoweringError> {
    let columns = match &select.columns {
        Columns::Every => SelectedColumns::All,
        Columns::Listed(listed) => {
            let mut items = Vec::with_capacity(listed.len());
            for item in listed {
                items.push(match (&item.column, &item.rename) {
                    (SelectedValue::Column(reference), Renaming::Unchanged) => SelectItem {
                        name: reference.column.clone(),
                        value: column(reference),
                    },
                    (SelectedValue::Column(reference), Renaming::Renamed(name)) => SelectItem {
                        name: name.clone(),
                        value: column(reference),
                    },
                    (SelectedValue::Applied(applied), Renaming::Renamed(name)) => SelectItem {
                        name: name.clone(),
                        value: function(applied, &scope.parameters)?,
                    },
                    (SelectedValue::Applied(_), Renaming::Unchanged) => {
                        return Err(PlanLoweringError::UnnamedColumn);
                    }
                });
            }
            SelectedColumns::Items(items)
        }
    };
    let mut sources = vec![source_step(expansion, scope, &select.from)?];
    if let Joining::Joined(join) = &select.join {
        sources.push(source_step(expansion, scope, &join.source)?);
    }
    let condition = select_condition(select, &scope.parameters)?;
    Ok(numbered(
        expansion,
        FinishStep::Select(SelectStep {
            sources,
            columns,
            condition,
            probe: ProbeMark::Unmarked,
            count: CountMark::Unmarked,
        }),
    ))
}

/// The arguments of a call, each name once: at its first place, with its last value.
fn deduplicated(arguments: &[Argument]) -> Vec<&Argument> {
    let mut kept: Vec<&Argument> = Vec::with_capacity(arguments.len());
    for argument in arguments {
        match kept.iter_mut().find(|earlier| earlier.name == argument.name) {
            Some(earlier) => *earlier = argument,
            None => kept.push(argument),
        }
    }
    kept
}

/// The step of a relation argument: a relation of the catalog (written in quotes), or a step the program named.
fn relation(
    expansion: &mut Expansion<'_>,
    scope: &ProgramScope<'_>,
    value: &ArgumentValue,
) -> Result<StepNumber, PlanLoweringError> {
    match value {
        ArgumentValue::Text(relation) => Ok(base(expansion, &relation.0)),
        ArgumentValue::Word(word) => named(scope, &word.0),
        ArgumentValue::Step(name) => named(scope, &name.0.0),
        ArgumentValue::Number(_)
        | ArgumentValue::Parameter(_)
        | ArgumentValue::List(_)
        | ArgumentValue::Map(_) => Err(PlanLoweringError::RelationArgument(WrittenText(
            format!("{value:?}").into(),
        ))),
    }
}

/// A call's step: the fragment it imports, inlined, or its operator's step.
fn called<'p>(
    expansion: &mut Expansion<'p>,
    scope: &ProgramScope<'p>,
    call: &'p Call,
) -> Result<StepNumber, PlanLoweringError> {
    let operator = &*call.name.0;
    if let Some(fragment) = scope.imports.get(operator) {
        let program = expansion
            .fragments
            .get(&QuerySourceId((*fragment).into()))
            .ok_or_else(|| PlanLoweringError::UnknownFragment(WrittenText((*fragment).into())))?;
        return inlined(expansion, scope, *fragment, program, &call.arguments);
    }
    let mut arguments = Vec::with_capacity(call.arguments.len());
    for argument in deduplicated(&call.arguments) {
        let name = &*argument.name.0;
        let value = if RELATION_ARGUMENTS.contains(&name) && !(name == ARG_FROM && operator == OP_ROWS)
        {
            StepArgument::Step(relation(expansion, scope, &argument.value)?)
        } else {
            StepArgument::Constant(constant(&argument.value, &scope.parameters)?)
        };
        arguments.push(UnlistedArgument {
            name: StepArgumentName(name.into()),
            value,
        });
    }
    let step = operator_step(operator, arguments)?;
    Ok(numbered(expansion, step))
}

/// A fragment inlined: its inputs the caller's relations, its parameters the caller's values (none when not given), its
/// steps lowered, and the step it returns.
fn inlined<'p>(
    expansion: &mut Expansion<'p>,
    caller: &ProgramScope<'p>,
    name: &'p str,
    fragment: &'p FragmentProgram,
    arguments: &'p [Argument],
) -> Result<StepNumber, PlanLoweringError> {
    let given = |name: &str| arguments.iter().rev().find(|argument| &*argument.name.0 == name);
    let mut instance = FragmentInstance {
        fragment: name,
        inputs: Vec::with_capacity(fragment.inputs.len()),
        values: Vec::with_capacity(fragment.parameters.len()),
    };
    for input in &fragment.inputs {
        let argument =
            given(&input.name.0).ok_or_else(|| PlanLoweringError::NotGiven(WrittenText(input.name.0.clone())))?;
        instance.inputs.push(relation(expansion, caller, &argument.value)?);
    }
    for declared in &fragment.parameters {
        instance.values.push(match given(&declared.parameter.0) {
            Some(argument) => constant(&argument.value, &caller.parameters)?,
            None => PlanConstant::Absent,
        });
    }
    if let Some(step) = expansion.instances.get(&instance) {
        return Ok(*step);
    }
    let mut scope = ProgramScope {
        imports: imported(&fragment.imports),
        ..ProgramScope::default()
    };
    for (input, step) in fragment.inputs.iter().zip(&instance.inputs) {
        let _earlier = scope.steps.insert(&*input.name.0, *step);
    }
    for (declared, value) in fragment.parameters.iter().zip(&instance.values) {
        let _earlier = scope.parameters.values.insert(declared.parameter.0.clone(), value.clone());
    }
    lets(expansion, &mut scope, &fragment.steps)?;
    let step = named(&scope, &fragment.result.step.0.0)?;
    let _new = expansion.instances.insert(instance, step);
    Ok(step)
}
