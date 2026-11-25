//! The `walk` contract's cases of growth: the language's fillers held to what the catalog declares of each relation they fill,
//! by the contract's growth harness, over generated sources of every shape at one size and at four times it. A whole fill's
//! rows grow with the file as the relation's `size` says, and a fill of a declared key answers that it wrote what was asked,
//! writes that key's rows alone, and writes no more for the larger file. No one shape shows every relation (a file the parser
//! refuses is the one that has syntax errors, and the one whose shapes cannot be told), so each shape says what it leaves
//! unproven, and every relation the language fills and every declared key is proven by some shape.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::grown::{GrownShape, SHAPES, grown_source};
use crate::fixtures::walking::walked;
use ryo::languages::contract::{GrowthFault, GrowthReport, grown};
use ryo::languages::{self, Count, NodeTable, Relation, Walk, WalkError};

/// The elements of the smaller file of a growth run.
const ELEMENTS: Count = Count(2_048);

/// What a case asks of the fillers' growth.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// What the harness finds for the sources of one shape.
    Shape(GrownShape),
    /// The relations the language answers that no shape proves, and the declared keys no shape proves.
    Unproven,
    /// How many elements a source of each shape has when a size is asked for.
    Sizes,
}

type Input = Asked;
/// What was found, as one text.
type Value = &'static str;
type Failure = WalkError;

/// The rows of the contract's cases of growth.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("statements_grow_as_the_catalog_declares"),
        word: Word::Succeeds,
        input: || Asked::Shape(GrownShape::Statements),
        expect: Expect::Value(
            "1981 and 7921 elements: 47 relations and 14 keys held, unproven [SyntaxError], faults []",
        ),
    },
    Case {
        name: CaseName("scopes_in_scopes_grow_as_the_catalog_declares"),
        word: Word::Succeeds,
        input: || Asked::Shape(GrownShape::Scopes),
        expect: Expect::Value(
            "2003 and 8163 elements: 27 relations and 11 keys held, unproven [Annotates, Assigns, Comment, Decorates, Inherits, BindingShadow, InterpolatedStringLiteral, FormatParse, FormatField, BraceFormatParse, BraceFormatField, RawFormatSegment, RawFormatField, LiteralExpressionParse, StaticImport, StandardLibraryImport, StringLiteral, BytesLiteral, FormatSpecification, SyntaxError, EmbeddedExpression], faults []",
        ),
    },
    Case {
        name: CaseName("one_long_expression_grows_as_the_catalog_declares"),
        word: Word::Succeeds,
        input: || Asked::Shape(GrownShape::Expression),
        expect: Expect::Value(
            "2052 and 8172 elements: 26 relations and 10 keys held, unproven [Function, Annotates, Assigns, Comment, Decorates, Inherits, Parameter, Resolves, BindingUse, BindingResolution, BindingShadow, FunctionTerminal, InterpolatedStringLiteral, FormatField, BraceFormatField, RawFormatField, StaticImport, StandardLibraryImport, BytesLiteral, FormatSpecification, SyntaxError, EmbeddedExpression], faults []",
        ),
    },
    Case {
        name: CaseName("many_names_in_one_scope_grow_as_the_catalog_declares"),
        word: Word::Succeeds,
        input: || Asked::Shape(GrownShape::Names),
        expect: Expect::Value(
            "2047 and 8189 elements: 27 relations and 12 keys held, unproven [Annotates, Assigns, Comment, Decorates, Inherits, Parameter, InterpolatedStringLiteral, FormatParse, FormatField, BraceFormatParse, BraceFormatField, RawFormatSegment, RawFormatField, LiteralExpressionParse, StaticImport, StandardLibraryImport, StringLiteral, BytesLiteral, FormatSpecification, SyntaxError, EmbeddedExpression], faults []",
        ),
    },
    Case {
        name: CaseName("refused_statements_grow_as_the_catalog_declares"),
        word: Word::Succeeds,
        input: || Asked::Shape(GrownShape::Errors),
        expect: Expect::Value(
            "2042 and 8191 elements: 10 relations and 4 keys held, unproven [Function, Annotates, Assigns, Comment, Decorates, Inherits, Parameter, StaticCall, CallArgument, Resolves, BindingEvent, BindingRead, BindingUse, BindingResolution, LookupScope, BindingShadow, QualifiedName, Identifier, ExpressionShape, LiteralProperties, FunctionTerminal, ExpressionType, InterpolatedStringLiteral, ConstantKey, FormatParse, FormatField, BraceFormatParse, BraceFormatField, RawFormatSegment, RawFormatField, LiteralExpressionParse, StaticImport, StandardLibraryImport, StringLiteral, BytesLiteral, FormatSpecification, EmbeddedExpression, NumericLiteral], faults []",
        ),
    },
    Case {
        name: CaseName("every_relation_and_every_declared_key_is_proven_by_some_shape"),
        word: Word::Succeeds,
        input: || Asked::Unproven,
        expect: Expect::Value(
            "48 relations answered, never proven []; 14 declared keys, never asked with a row []",
        ),
    },
    Case {
        name: CaseName("a_grown_source_has_about_the_elements_asked_for"),
        word: Word::Succeeds,
        input: || Asked::Sizes,
        expect: Expect::Value(
            "Statements 1981, Scopes 2003, Expression 2052, Names 2047, Errors 2042",
        ),
    },
];

/// The growth harness over the sources of one shape, walked by `walk`: at `ELEMENTS` elements and at four times as many.
fn report<W: Walk>(walk: &W, shape: GrownShape) -> GrowthReport {
    let (mut table, mut scratch) = (NodeTable::default(), W::Scratch::default());
    grown(&W::vocabulary().fixed, ELEMENTS, &mut |asked, visit| {
        let source = grown_source(shape, asked);
        // A file the language does not walk lends nothing, and the harness says so.
        let _walked = walked(
            walk,
            "grown.py",
            source.as_bytes(),
            languages::all(),
            &mut table,
            &mut scratch,
            visit,
        );
    })
}

/// The relations a report leaves unproven.
fn unproven(report: &GrowthReport) -> Vec<Relation> {
    report
        .faults
        .iter()
        .filter_map(|fault| match fault {
            GrowthFault::Unproven(relation) => Some(*relation),
            GrowthFault::FixedNames(_)
            | GrowthFault::Sources
            | GrowthFault::NothingAnswered
            | GrowthFault::Grew(_)
            | GrowthFault::KeyNotAsked(_)
            | GrowthFault::KeyRows(_)
            | GrowthFault::KeyGrew(_) => None,
        })
        .collect()
}

/// What the harness finds for one shape: the two sizes, how many relations hold their declared size and how many declared
/// keys were asked, the relations the shape leaves unproven, and every other fault.
fn shape_told(shape: GrownShape) -> String {
    let report = report(&crate::walk(), shape);
    let left = unproven(&report);
    let faults: Vec<&GrowthFault> = report
        .faults
        .iter()
        .filter(|fault| !matches!(fault, GrowthFault::Unproven(_)))
        .collect();
    format!(
        "{} and {} elements: {} relations and {} keys held, unproven {left:?}, faults {faults:?}",
        report.small.0,
        report.large.0,
        report
            .relations
            .iter()
            .filter(|growth| !left.contains(&growth.relation))
            .count(),
        report.keys.len(),
    )
}

/// The relations the language answers that every shape leaves unproven, and the declared keys of the relations it answers
/// that no shape asks with a row at both sizes.
fn never_proven() -> String {
    let reports: Vec<GrowthReport> = SHAPES
        .iter()
        .map(|shape| report(&crate::walk(), *shape))
        .collect();
    let mut relations: Vec<Relation> = Vec::new();
    let mut keys = Vec::new();
    for each in &reports {
        relations.extend(unproven(each));
        relations.extend(each.relations.iter().map(|growth| growth.relation));
        keys.extend(each.keys.iter().map(|key| key.column));
    }
    relations.sort_unstable_by_key(|relation| format!("{relation:?}"));
    relations.dedup();
    keys.sort_unstable_by_key(|column| format!("{column:?}"));
    keys.dedup();
    let proven_by = |relation: Relation| {
        reports.iter().any(|each| {
            each.relations
                .iter()
                .any(|growth| growth.relation == relation)
                && !unproven(each).contains(&relation)
        })
    };
    let lost: Vec<Relation> = relations
        .iter()
        .copied()
        .filter(|relation| !proven_by(*relation))
        .collect();
    let asked = |column| {
        reports.iter().any(|each| {
            each.keys
                .iter()
                .any(|key| key.column == column && key.small.0 > 0 && key.large.0 > 0)
        })
    };
    let unasked: Vec<_> = keys
        .iter()
        .copied()
        .filter(|column| !asked(*column))
        .collect();
    format!(
        "{} relations answered, never proven {lost:?}; {} declared keys, never asked with a row {unasked:?}",
        relations.len(),
        keys.len()
    )
}

/// The elements of a source of each shape when `ELEMENTS` are asked for.
fn sizes() -> Result<String, WalkError> {
    let walk = crate::walk();
    let (mut table, mut scratch) = (NodeTable::default(), Default::default());
    let mut told = Vec::new();
    for shape in SHAPES {
        let source = grown_source(shape, ELEMENTS);
        walked(
            &walk,
            "grown.py",
            source.as_bytes(),
            &[],
            &mut table,
            &mut scratch,
            &mut |file| {
                told.push(format!(
                    "{shape:?} {}",
                    languages::node_table::element_count(file.table).0
                ));
            },
        )?;
    }
    Ok(told.join(", "))
}

fn subject(asked: Input) -> Result<Value, Failure> {
    let told = match asked {
        Asked::Shape(shape) => shape_told(shape),
        Asked::Unproven => never_proven(),
        Asked::Sizes => sizes()?,
    };
    Ok(Box::leak(told.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
