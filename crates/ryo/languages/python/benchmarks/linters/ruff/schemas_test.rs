//! UTF-8 offsets must not be confused with Ruff's scalar columns.
use super::schemas::{Location, offset};
#[test]
fn unicode_and_crlf() {
    assert_eq!(
        offset("é = 1\r\npass\n", &Location { row: 1, column: 2 }).unwrap(),
        2
    );
    assert_eq!(
        offset("é = 1\r\npass\n", &Location { row: 2, column: 1 }).unwrap(),
        8
    );
    assert_eq!(
        offset("pass\n", &Location { row: 2, column: 1 }).unwrap(),
        5
    );
}
#[test]
fn invalid_coordinates_fail() {
    for location in [
        Location { row: 0, column: 1 },
        Location { row: 2, column: 1 },
        Location { row: 1, column: 5 },
    ] {
        assert!(offset("é", &location).is_err());
    }
}

#[test]
fn bom_and_lone_carriage_return() {
    assert_eq!(
        offset("\u{feff}import os\n", &Location { row: 1, column: 8 }).unwrap(),
        10
    );
    assert_eq!(
        offset("\u{feff}import os\n", &Location { row: 1, column: 10 }).unwrap(),
        12
    );
    assert_eq!(
        offset(
            "x = 1\rtry:\r    pass\rexcept:\r    pass\r",
            &Location { row: 4, column: 1 }
        )
        .unwrap(),
        20
    );
    assert_eq!(
        offset("one\r\ntwo\rthree\nfour", &Location { row: 4, column: 1 }).unwrap(),
        15
    );
}
