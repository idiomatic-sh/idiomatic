//! The `expansion` service: a pack's programs as one plan.

use super::types::{Expansion, FragmentInstance, ProgramScope};
use ryo_values::domain::finish_plans::arguments::literals::types::{
    ArgumentValue as StepArgument, StepArgumentName, UnlistedArgument,
};
use ryo_values::domain::finish_plans::arguments::types::StepNumber;
use ryo_values::domain::finish_plans::constants::{
    ARG_FROM, ARG_LEFT, ARG_LEFT_ON, ARG_OUTPUT, ARG_RIGHT, ARG_RIGHT_ON, OP_ROWS, OP_UNION, RELATION_ARGUMENTS,
};
use ryo_values::domain::finish_plans::expressions::types::PlanConstant;
use ryo_values::domain::finish_plans::types::{
    CountMark, EmitSubject, FinishPlan, FinishStep, PlanEmit, PlannedRule, ProbeMark, RuleEmit,
    SelectItem, SelectSource, SelectStep, SelectedColumns, UnplannedReason, UnplannedRule,
};
use crate::domain::plan_lowering::context::service::{producer_select, table_arguments, union_arguments};
use crate::domain::plan_lowering::errors::PlanLoweringError;
use crate::domain::plan_lowering::joins::constants::JOIN;
use crate::domain::plan_lowering::joins::service::{JoinCall, joined};
use crate::domain::plan_lowering::numbering::service::numbered;
use crate::domain::plan_lowering::filters::service::{alias, column, constant, function, operand, select_condition};
use crate::domain::plan_lowering::operators::service::operator_step;
use crate::domain::plan_lowering::types::{PackPrograms, PackRule, RuleProgram, WrittenText};
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
        context: &pack.context,
        observed: hashbrown::HashMap::new(),
        observations_open: false,
    };
    let mut plan = FinishPlan {
        steps: Vec::new(),
        rules: Vec::new(),
        unplanned: Vec::new(),
    };
    // A rule that reads a rule observation before all its producers are lowered is lowered after every other rule, its
    // steps of the first try taken back.
    let mut deferred: Vec<&PackRule> = Vec::new();
    for rule in &pack.rules {
        let made = expansion.steps.len();
        let lowered = rule_lowered(&mut expansion, rule);
        if let Err(PlanLoweringError::UnknownStep(name)) = &lowered
            && pack.context.observations.iter().any(|seen| *seen.name.0 == *name.0)
        {
            taken_back(&mut expansion, made);
            deferred.push(rule);
            continue;
        }
        recorded(&mut plan, rule, lowered);
    }
    expansion.observations_open = true;
    for rule in deferred {
        let lowered = rule_lowered(&mut expansion, rule);
        recorded(&mut plan, rule, lowered);
    }
    plan.steps = expansion.steps;
    plan
}

/// A rule's emit, its observed outputs recorded for the rules that read them.
fn rule_lowered<'p>(expansion: &mut Expansion<'p>, rule: &'p PackRule) -> Result<RuleEmit, PlanLoweringError> {
    match &rule.program {
        RuleProgram::Parsed(program) => rule_emit(expansion, &rule.rule.0, program),
        RuleProgram::Unread(why) => Err(PlanLoweringError::Unread(WrittenText(why.0.clone()))),
    }
}

/// The steps made from `made` on taken back, with every number and fragment instance that names one of them.
fn taken_back(expansion: &mut Expansion<'_>, made: usize) {
    expansion.steps.truncate(made);
    expansion.numbers.retain(|number| number.0 < made);
    expansion.instances.retain(|_, step| step.0 < made);
    expansion.observed.retain(|_, step| step.0 < made);
}

/// A rule's lowering in the plan: planned, or unplanned with its refusal.
fn recorded(plan: &mut FinishPlan, rule: &PackRule, lowered: Result<RuleEmit, PlanLoweringError>) {
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

/// A rule's program lowered: the step it emits.
fn rule_emit<'p>(
    expansion: &mut Expansion<'p>,
    rule: &'p str,
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
    for observation in &program.observations {
        let step = named(&scope, &observation.step.0.0)?;
        let _earlier = expansion.observed.insert((rule, &*observation.name.0), step);
    }
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

/// The step of a relation of the catalog, or the rows of the context table of that name.
fn base(expansion: &mut Expansion<'_>, relation: &str) -> Result<StepNumber, PlanLoweringError> {
    let table = expansion.context.tables.iter().find(|table| *table.relation.0 == *relation);
    let step = match table {
        Some(table) => operator_step(OP_ROWS, table_arguments(table))?,
        None => FinishStep::Base(RelationName(relation.into())),
    };
    Ok(numbered(expansion, step))
}

/// The step a name read as a relation names: the program's, else the rule observation of that name (the union of its
/// producers' outputs lowered so far; unknown while none is).
fn step_named(
    expansion: &mut Expansion<'_>,
    scope: &ProgramScope<'_>,
    name: &str,
) -> Result<StepNumber, PlanLoweringError> {
    if let Ok(step) = named(scope, name) {
        return Ok(step);
    }
    let context = expansion.context;
    let Some(seen) = context.observations.iter().find(|seen| *seen.name.0 == *name).filter(|_| expansion.observations_open)
    else {
        return named(scope, name);
    };
    let mut united: Option<StepNumber> = None;
    for producer in &seen.producers {
        let Some(output) = expansion.observed.get(&(&*producer.rule.0, &*producer.output.0)).copied() else {
            continue;
        };
        let part = numbered(expansion, producer_select(output, &seen.columns));
        united = Some(match united {
            None => part,
            Some(left) => {
                let union = operator_step(OP_UNION, union_arguments(left, part))?;
                numbered(expansion, union)
            }
        });
    }
    united.ok_or_else(|| PlanLoweringError::UnknownStep(WrittenText(name.into())))
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
        SourceReference::Relation(relation) => base(expansion, &relation.0)?,
        SourceReference::Step(name) => step_named(expansion, scope, &name.0.0)?,
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
                    (SelectedValue::Constant(constant), Renaming::Renamed(name)) => SelectItem {
                        name: name.clone(),
                        value: operand(constant, &scope.parameters)?,
                    },
                    (SelectedValue::Applied(_) | SelectedValue::Constant(_), Renaming::Unchanged) => {
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
        ArgumentValue::Text(relation) => base(expansion, &relation.0),
        ArgumentValue::Word(word) => step_named(expansion, scope, &word.0),
        ArgumentValue::Step(name) => step_named(expansion, scope, &name.0.0),
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
    if operator == JOIN {
        return join_called(expansion, scope, &call.arguments);
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

/// A `join` call's select (the `joins` concept); a missing argument refuses it.
fn join_called<'p>(
    expansion: &mut Expansion<'p>,
    scope: &ProgramScope<'p>,
    arguments: &'p [Argument],
) -> Result<StepNumber, PlanLoweringError> {
    let given = |name: &str| {
        arguments
            .iter()
            .rev()
            .find(|argument| &*argument.name.0 == name)
            .map(|argument| &argument.value)
            .ok_or_else(|| PlanLoweringError::NotGiven(WrittenText(name.into())))
    };
    let left = relation(expansion, scope, given(ARG_LEFT)?)?;
    let right = relation(expansion, scope, given(ARG_RIGHT)?)?;
    let left_on = constant(given(ARG_LEFT_ON)?, &scope.parameters)?;
    let right_on = constant(given(ARG_RIGHT_ON)?, &scope.parameters)?;
    let output = constant(given(ARG_OUTPUT)?, &scope.parameters)?;
    let step = joined(&JoinCall { left, right, left_on: &left_on, right_on: &right_on, output: &output })?;
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
