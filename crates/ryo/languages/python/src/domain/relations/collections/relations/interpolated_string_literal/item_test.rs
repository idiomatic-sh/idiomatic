//! The reader emits decoded f-string text pieces, not interpolation or format-spec text.

use super::super::fixtures::contract::check;
use super::item::InterpolatedStringLiteralItem;
use crate::fixtures::reading::reading;
use ryo::languages::contract::Want;

#[test]
fn fstring_fragments_are_scoped_and_decoded() {
    check::<InterpolatedStringLiteralItem>();
    let result = reading::<InterpolatedStringLiteralItem>(concat!(
        "from datetime import datetime\n",
        "datetime.strptime('2024', f'%Y %z {zone}')\n",
        "datetime.strptime('2024', f'{zone} %z')\n",
        "datetime.strptime('2024', f\"{'%z'}\")\n",
        "datetime.strptime('2024', f'{zone:%z}')\n",
        "datetime.strptime('2024', f'{f\"%z\"}')\n",
    ));
    assert!(result.causes.is_empty());
    let rows = result.rows;
    let values = rows
        .iter()
        .map(|row| match row[2] {
            Want::Name(value) => value,
            _ => panic!("fragment value is decoded text"),
        })
        .collect::<Vec<_>>();
    assert_eq!(values, ["%Y %z ", " %z", "%z"]);
    assert_ne!(rows[0][1], rows[1][1], "owners are separate strings");
    assert_ne!(
        rows[0][1], rows[2][1],
        "nested f-strings keep their own owner"
    );
    assert_ne!(
        rows[1][1], rows[2][1],
        "nested f-strings keep their own owner"
    );
}
