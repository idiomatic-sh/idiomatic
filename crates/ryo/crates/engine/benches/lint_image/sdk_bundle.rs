/// Transfer the immutable module sources across the linked SDK version boundary.
pub fn query_modules(
    query: &crate::query::RyoQlQuery,
) -> Option<std::sync::Arc<std::collections::BTreeMap<Box<str>, std::sync::Arc<str>>>> {
    let bundle = query.clone().into_bundle();
    if bundle.modules.is_empty() {
        return None;
    }
    Some(std::sync::Arc::new(
        bundle
            .modules
            .iter()
            .map(|(id, document)| (id.0.clone(), document.text.0.clone()))
            .collect(),
    ))
}
