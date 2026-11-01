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
    let owners = rows
        .iter()
        .map(|row| match row {
            [_, owner, ..] => *owner,
            _ => panic!("literal fragment rows include their owner"),
        })
        .collect::<Vec<_>>();
    let [
        first,
        second,
        third,
        nested,
        outer_prefix,
        inner,
        outer_suffix,
    ] = owners.as_slice()
    else {
        panic!("the sample has seven literal fragments");
    };
    assert_ne!(first, second, "owners are separate strings");
    assert_ne!(first, nested, "nested f-strings keep their own owner");
    assert_ne!(second, nested, "nested f-strings keep their own owner");
    assert_ne!(third, nested, "nested f-strings keep their own owner");
    assert_eq!(
        outer_prefix, outer_suffix,
        "outer fragments share their owner"
    );
    assert_ne!(
        outer_prefix, inner,
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
