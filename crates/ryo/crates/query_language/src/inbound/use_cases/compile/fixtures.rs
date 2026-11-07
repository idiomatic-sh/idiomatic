//! The test support of the `compile` use case: the scenario its tests share, built over the fakes of its ports.

use crate::outbound::catalog::fixtures::fake::CatalogFake;
use crate::outbound::syntax::fixtures::fake::SyntaxFake;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::descriptions::types::ColumnName;
use ryo_values::domain::hosted::types::HostedRelation;
use ryo_values::domain::observations::types::BoundObservationInput;
use ryo_values::domain::program::types::{LetStep, WrittenEmit, WrittenProgram};
use ryo_values::domain::query_syntax::types::{
    Alias, Columns, Condition, Expression, Filter, Joining, QuerySource, Select, SourceReference,
    WrittenOperand, WrittenTest,
};
use ryo_values::domain::query_tokens::types::{
    ByteOffset, QuerySpan, StepName, WrittenLiteral, WrittenRelation,
};

pub(super) const NOWHERE: QuerySpan = QuerySpan {
    start: ByteOffset(0),
    end: ByteOffset(0),
};


/// What a row gives the use case: the ports it calls, and the text.
pub(super) struct Given {
    pub(super) syntax: SyntaxFake,
    pub(super) catalog: CatalogFake,
    pub(super) text: &'static str,
    pub(super) observations: Vec<BoundObservationInput>,
    pub(super) hosted: Vec<HostedRelation>,
}

pub(super) fn read_function(filters: Vec<Filter>) -> LetStep {
    LetStep {
        name: StepName(
            "functions".into(),
            ryo_values::domain::query_tokens::types::StepIdentified::Unassigned,
        ),
        expression: Expression::Select(Select {
            columns: Columns::Every,
            from: QuerySource {
                reference: SourceReference::Relation(WrittenRelation("function".into())),
                alias: Alias::Unwritten,
                span: NOWHERE,
            },
            join: Joining::Alone,
            filters: filters.into_iter().map(Condition::Test).collect(),
        }),
        span: NOWHERE,
    }
}

pub(super) fn program_with(filters: Vec<Filter>) -> WrittenProgram {
    WrittenProgram {
        linked: ryo_values::domain::query_tokens::types::Linking::Unlinked,
        imports: Vec::new(),
        origins: Vec::new(),
        fragment_contracts: Vec::new(),
        step_origins: std::collections::BTreeMap::new(),
        observations: Vec::new(),
        emit_origin: ryo_values::domain::query_tokens::types::Attribution::Unlocated,
        parameters: Vec::new(),
        steps: vec![read_function(filters)],
        emit: WrittenEmit {
            step: StepName(
                "functions".into(),
                ryo_values::domain::query_tokens::types::StepIdentified::Unassigned,
            ),
            subject: ColumnName("file".into()),
            key: vec![ColumnName("span".into())],
            span: NOWHERE,
        },
    }
}

pub(super) fn text_the_fake_reads() -> Given {
    Given {
        syntax: SyntaxFake {
            program: program_with(Vec::new()),
        },
        catalog: CatalogFake::default(),
        text: "emit",
        observations: Vec::new(),
        hosted: Vec::new(),
    }
}

pub(super) fn text_the_fake_refuses() -> Given {
    Given {
        syntax: SyntaxFake::default(),
        catalog: CatalogFake::default(),
        text: "let a = ;",
        observations: Vec::new(),
        hosted: Vec::new(),
    }
}

pub(super) fn catalog_without_relations() -> Given {
    Given {
        catalog: CatalogFake {
            relations: Vec::new(),
            fails: false,
        },
        ..text_the_fake_reads()
    }
}

pub(super) fn comparing_text_with_a_number() -> Given {
    let filter = Filter {
        left: WrittenOperand::Text(WrittenLiteral("a".into())),
        test: WrittenTest::Equal,
        right: WrittenOperand::Number(Integer(1)),
        span: NOWHERE,
    };
    Given {
        syntax: SyntaxFake {
            program: program_with(vec![filter]),
        },
        ..text_the_fake_reads()
    }
}

pub(super) fn emitting_a_step_nobody_defined() -> Given {
    let mut program = program_with(Vec::new());
    program.emit.step = StepName(
        "later".into(),
        ryo_values::domain::query_tokens::types::StepIdentified::Unassigned,
    );
    Given {
        syntax: SyntaxFake { program },
        ..text_the_fake_reads()
    }
}

pub(super) fn failing_catalog() -> Given {
    Given {
        catalog: CatalogFake {
            relations: Vec::new(),
            fails: true,
        },
        ..text_the_fake_reads()
    }
}

pub(super) fn same_text_twice() -> Given {
    text_the_fake_reads()
}
