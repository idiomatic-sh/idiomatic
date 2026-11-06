//! The `descriptions` service: free functions named by verb over what ryo says of the relations it owns, with no IO and no ports.

use crate::domain::descriptions::types::ColumnDescription;
use crate::domain::descriptions::types::ColumnKind;
use crate::domain::descriptions::types::KindWord;
use crate::domain::descriptions::types::RelationDescription;
use crate::domain::descriptions::types::RelationName;
use crate::domain::descriptions::types::RelationSummary;
use crate::domain::descriptions::types::ColumnName;
use ryo_relations::domain::relations::service::vocabulary_of;
use ryo_relations::domain::relations::constants::kind_name;
use ryo_relations::domain::relations::service::column_name;
use ryo_relations::domain::relations::service::columns;
use ryo_relations::domain::relations::service::kind;
use ryo_relations::domain::relations::service::name;
use ryo_relations::domain::relations::service::summary;
use ryo_relations::domain::relations::types::Relation;
use ryo_relations::domain::terms::service::words_of;
use ryo_relations::domain::terms::types::Vocabulary;

/// One line about a relation.
#[must_use]
pub fn summarize(relation: Relation) -> RelationSummary {
    RelationSummary {
        name: RelationName(name(relation).0.into()),
        summary: summary(relation),
    }
}

/// A relation and its columns, for a person to read.
#[must_use]
pub fn describe(relation: Relation) -> RelationDescription {
    RelationDescription {
        languages: Vec::new(),
        name: RelationName(name(relation).0.into()),
        summary: summary(relation),
        columns: columns(relation)
            .into_iter()
            .map(|column| ColumnDescription {
                name: ColumnName(column_name(column).0.into()),
                kind: ColumnKind(kind_name(kind(column)).into()),
                words: match vocabulary_of(column) {
                    Vocabulary::Closed(block) => words_of(block)
                        .into_iter()
                        .map(|word| KindWord(word.0.into()))
                        .collect(),
                    Vocabulary::Open => Vec::new(),
                },
            })
            .collect(),
    }
}
