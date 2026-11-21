//! The names of this concept.
/// The text `builtin`.
pub(crate) const BUILTIN: &str = crate::domain::syntax::constants::BUILTIN;
/// The text `module-global`.
pub(crate) const MODULE_GLOBAL: &str = "module-global";
/// The text `WindowsError`.
pub(crate) const WINDOWSERROR: &str = "WindowsError";
/// The text `platform-builtin`.
pub(crate) const PLATFORM_BUILTIN: &str = "platform-builtin";
/// The text `__annotations__`.
pub(crate) const ANNOTATIONS: &str = "__annotations__";
/// The text `__builtins__`.
pub(crate) const BUILTINS: &str = "__builtins__";
/// The text `__cached__`.
pub(crate) const CACHED: &str = "__cached__";
/// The text `__doc__`.
pub(crate) const DOC: &str = "__doc__";
/// The text `__file__`.
pub(crate) const FILE: &str = "__file__";
/// The text `__loader__`.
pub(crate) const LOADER: &str = "__loader__";
/// The text `__name__`.
pub(crate) const NAME: &str = "__name__";
/// The text `__package__`.
pub(crate) const PACKAGE: &str = "__package__";
/// The text `__spec__`.
pub(crate) const SPEC: &str = "__spec__";
/// The text `__warningregistry__`.
pub(crate) const WARNINGREGISTRY: &str = "__warningregistry__";
/// The text `__annotate__`.
pub(crate) const ANNOTATE: &str = "__annotate__";
/// The kinds of an implicit name and the one name of a platform, as a filler writes them: the same in every file, so fixed names
/// of the run. The names of the catalogs are fixed names too, read from the catalogs.
pub(crate) const FIXED_NAMES: &[&str] = &[BUILTIN, MODULE_GLOBAL, WINDOWSERROR, PLATFORM_BUILTIN];
