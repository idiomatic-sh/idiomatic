//! Names used to bind a reader’s pair program.
use super::super::first;
use crate::languages as l;

pub(crate) fn vocabulary(kinds: &[l::NameText<'_>], fields: l::FieldNames) -> first::Vocab {
    fn codes(names: &[l::NameText<'_>]) -> std::collections::HashMap<String, u16> {
        let mut codes = std::collections::HashMap::new();
        let mut ambiguous = std::collections::HashSet::new();
        for (i, name) in names.iter().enumerate() {
            if codes
                .insert(
                    name.0.to_owned(),
                    u16::try_from(i).expect("vocabulary code"),
                )
                .is_some()
            {
                ambiguous.insert(name.0);
            }
        }
        codes.retain(|name, _| !ambiguous.contains(name.as_str()));
        codes
    }
    first::Vocab {
        kinds: codes(kinds),
        kind_names: kinds.iter().map(|n| n.0.to_owned()).collect(),
        fields: codes(&fields.0),
        field_names: fields.0.iter().map(|n| n.0.to_owned()).collect(),
    }
}
