//! The reader emits decoded f-string text pieces, not interpolation or format-spec text.

use super::super::fixtures::contract::check;
use super::item::InterpolatedStringLiteralItem;
use crate::fixtures::reading::reading;
use ryo::languages::contract::Want;

#[test]
fn fstring_fragments_are_scoped_and_decoded() {
    check::<InterpolatedStringLiteralItem>();
    let source = concat!(
        "from datetime import datetime\n",
        "datetime.strptime('2024', f'%Y %z {zone}')\n",
        "datetime.strptime('2024', f'{zone} %z')\n",
        "datetime.strptime('2024', f'{zone}' ' %z')\n",
        "datetime.strptime('2024', f\"{'%z'}\")\n",
        "datetime.strptime('2024', f'{zone:%z}')\n",
        "datetime.strptime('2024', f'{f\"%z\"}')\n",
        "datetime.strptime('2024', f'prefix {f\"%z\"} suffix')\n",
    );
    let result = reading::<InterpolatedStringLiteralItem>(source);
    assert!(result.causes.is_empty(), "{:?}", result.causes);
    let rows = result.rows;
    let values = rows
        .iter()
        .map(|row| match row[2] {
            Want::Name(value) => value,
            Want::Unlocated
            | Want::Node(_)
            | Want::Unknown(_)
            | Want::File(_)
            | Want::Span(..)
            | Want::Number(_)
            | Want::Flag(_) => panic!("fragment value is decoded text"),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        values,
        ["%Y %z ", " %z", " %z", "%z", "prefix ", "%z", " suffix",]
    );
    assert_ne!(rows[0][1], rows[1][1], "owners are separate strings");
    assert_ne!(
        rows[0][1], rows[3][1],
        "nested f-strings keep their own owner"
    );
    assert_ne!(
        rows[1][1], rows[3][1],
        "nested f-strings keep their own owner"
    );
    assert_ne!(
        rows[2][1], rows[3][1],
        "nested f-strings keep their own owner"
    );
    assert_eq!(rows[4][1], rows[6][1], "outer fragments share their owner");
    assert_ne!(
        rows[4][1], rows[5][1],
        "nested fragments have a separate owner"
    );
    let adjacent =
        reading::<InterpolatedStringLiteralItem>("datetime.strptime('2024', f'{zone}' ' %z')\n");
    assert!(adjacent.causes.is_empty());
    assert!(
        adjacent.rows.iter().any(|row| row[2] == Want::Name(" %z")),
        "{:?}",
        adjacent.rows
    );
}
