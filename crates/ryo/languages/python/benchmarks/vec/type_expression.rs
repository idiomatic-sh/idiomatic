//! Pack inputs for the Python provider in the reference image.
pub(super) use ryo_python::fixtures::named_inputs::Lists;

pub(super) static LISTS: std::sync::OnceLock<Lists> = std::sync::OnceLock::new();

pub(super) fn load_lists(pack: &str) {
    let _ = LISTS.set(Lists::load(std::path::Path::new(pack)).expect("type-expression declarations"));
}
