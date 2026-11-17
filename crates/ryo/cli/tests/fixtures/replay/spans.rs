//! The span a composed row gives, held to the start and end its witness selects on its own: the Python feature contracts check every
//! `span-between-witness` row with it, and its own test holds the check to both boundaries.

/// The replay renders span cells as byte_start..byte_end; compare every actual composed span
/// with independently selected child-start and parent-end source locations, including delimiters.
pub(crate) fn verify_composed_spans(rows: &[Vec<String>]) -> Result<(), String> {
    for row in rows {
        if row
            .first()
            .is_none_or(|rule| rule != "span-between-witness")
        {
            continue;
        }
        let [_, _, key, _, _] = row.as_slice() else {
            return Err("malformed span witness row".into());
        };
        let mut parts = key.split(',');
        let Some(((_id, begin), (finish, span))) = parts
            .next()
            .zip(parts.next())
            .zip(parts.next().zip(parts.next()))
        else {
            return Err("malformed span witness key".into());
        };
        if parts.next().is_some() {
            return Err("malformed span witness key".into());
        }
        let begin: u32 = begin.parse().map_err(|_| "invalid span start")?;
        let finish: u32 = finish.parse().map_err(|_| "invalid span end")?;
        let Some((actual_begin, actual_finish)) = span.split_once("..") else {
            return Err("malformed composed span".into());
        };
        let actual_begin: u32 = actual_begin.parse().map_err(|_| "invalid composed start")?;
        let actual_finish: u32 = actual_finish.parse().map_err(|_| "invalid composed end")?;
        if begin >= finish || actual_begin != begin || actual_finish != finish {
            return Err(format!("span-between endpoints differ: {key}"));
        }
    }
    Ok(())
}

/// These are rendered-cell unit examples, not source rows added to the measured feed counts.
#[test]
fn composed_span_witness_checks_both_boundaries_and_nonempty_extent() {
    let row = |key: &str| {
        vec![
            "span-between-witness".to_owned(),
            "input.py".to_owned(),
            key.to_owned(),
            "1".to_owned(),
            "1".to_owned(),
        ]
    };
    assert!(verify_composed_spans(&[row("id,2,9,2..9")]).is_ok());
    for key in [
        "id,2,9,1..9",
        "id,2,9,2..8",
        "id,9,2,9..2",
        "id,2,2,2..2",
        "id,2,9,missing",
        "id,2,9,2..9,extra",
    ] {
        assert!(verify_composed_spans(&[row(key)]).is_err(), "{key}");
    }
}
