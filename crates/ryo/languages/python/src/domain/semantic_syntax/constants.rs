//! Vocabulary of Python typing contexts and physical source mapping.
/// The `typing.` typing or mapping vocabulary.
pub(crate) const TYPING: &str = "typing.";
/// The `typing_extensions.` typing or mapping vocabulary.
pub(crate) const TYPING_EXTENSIONS: &str = "typing_extensions.";
/// The `Literal` typing or mapping vocabulary.
pub(crate) const LITERAL: &str = "Literal";
/// The `Annotated` typing or mapping vocabulary.
pub(crate) const ANNOTATED: &str = "Annotated";
/// The `cast` typing or mapping vocabulary.
pub(crate) const CAST: &str = "cast";
/// The `NewType` typing or mapping vocabulary.
pub(crate) const NEW_TYPE: &str = "NewType";
/// The `TypeVar` typing or mapping vocabulary.
pub(crate) const TYPE_VAR: &str = "TypeVar";
/// The `TypeAlias` typing or mapping vocabulary.
pub(crate) const TYPE_ALIAS: &str = "TypeAlias";
/// The `typ` typing or mapping vocabulary.
pub(crate) const TYPE_KEYWORD: &str = "typ";
/// The `tp` typing or mapping vocabulary.
pub(crate) const NEW_TYPE_KEYWORD: &str = "tp";
/// The `bound` typing or mapping vocabulary.
pub(crate) const BOUND: &str = "bound";
/// The `default` typing or mapping vocabulary.
pub(crate) const DEFAULT: &str = "default";
/// The `deferred-type-expression` typing or mapping vocabulary.
pub(crate) const DEFERRED_CONTEXT: &str = "deferred-type-expression";
/// The `exact-content` typing or mapping vocabulary.
pub(crate) const EXACT_MAPPING: &str = "exact-content";
/// The `whole-host` typing or mapping vocabulary.
pub(crate) const HOST_MAPPING: &str = "whole-host";

/// Resolved Python constructor vocabulary: `builtins.`.
pub(super) const BUILTINS: &str = "builtins.";
/// Resolved Python constructor vocabulary: `dict`.
pub(super) const BUILTIN_DICT: &str = "dict";
/// Resolved Python constructor vocabulary: `frozenset`.
pub(super) const BUILTIN_FROZENSET: &str = "frozenset";
/// Resolved Python constructor vocabulary: `list`.
pub(super) const BUILTIN_LIST: &str = "list";
/// Resolved Python constructor vocabulary: `set`.
pub(super) const BUILTIN_SET: &str = "set";
/// Resolved Python constructor vocabulary: `tuple`.
pub(super) const BUILTIN_TUPLE: &str = "tuple";
/// Resolved Python constructor vocabulary: `type`.
pub(super) const BUILTIN_TYPE: &str = "type";
/// Resolved Python constructor vocabulary: `AbstractSet`.
pub(super) const ABSTRACT_SET: &str = "AbstractSet";
/// Resolved Python constructor vocabulary: `AsyncContextManager`.
pub(super) const ASYNC_CONTEXT_MANAGER: &str = "AsyncContextManager";
/// Resolved Python constructor vocabulary: `AsyncGenerator`.
pub(super) const ASYNC_GENERATOR: &str = "AsyncGenerator";
/// Resolved Python constructor vocabulary: `AsyncIterable`.
pub(super) const ASYNC_ITERABLE: &str = "AsyncIterable";
/// Resolved Python constructor vocabulary: `AsyncIterator`.
pub(super) const ASYNC_ITERATOR: &str = "AsyncIterator";
/// Resolved Python constructor vocabulary: `Awaitable`.
pub(super) const AWAITABLE: &str = "Awaitable";
/// Resolved Python constructor vocabulary: `Callable`.
pub(super) const CALLABLE: &str = "Callable";
/// Resolved Python constructor vocabulary: `ChainMap`.
pub(super) const CHAIN_MAP: &str = "ChainMap";
/// Resolved Python constructor vocabulary: `ClassVar`.
pub(super) const CLASS_VAR: &str = "ClassVar";
/// Resolved Python constructor vocabulary: `Collection`.
pub(super) const COLLECTION: &str = "Collection";
/// Resolved Python constructor vocabulary: `Concatenate`.
pub(super) const CONCATENATE: &str = "Concatenate";
/// Resolved Python constructor vocabulary: `Container`.
pub(super) const CONTAINER: &str = "Container";
/// Resolved Python constructor vocabulary: `ContextManager`.
pub(super) const CONTEXT_MANAGER: &str = "ContextManager";
/// Resolved Python constructor vocabulary: `Coroutine`.
pub(super) const COROUTINE: &str = "Coroutine";
/// Resolved Python constructor vocabulary: `Counter`.
pub(super) const COUNTER: &str = "Counter";
/// Resolved Python constructor vocabulary: `DefaultDict`.
pub(super) const DEFAULT_DICT: &str = "DefaultDict";
/// Resolved Python constructor vocabulary: `Deque`.
pub(super) const DEQUE: &str = "Deque";
/// Resolved Python constructor vocabulary: `Dict`.
pub(super) const DICT: &str = "Dict";
/// Resolved Python constructor vocabulary: `Final`.
pub(super) const FINAL: &str = "Final";
/// Resolved Python constructor vocabulary: `FrozenSet`.
pub(super) const FROZEN_SET: &str = "FrozenSet";
/// Resolved Python constructor vocabulary: `Generator`.
pub(super) const GENERATOR: &str = "Generator";
/// Resolved Python constructor vocabulary: `Generic`.
pub(super) const GENERIC: &str = "Generic";
/// Resolved Python constructor vocabulary: `IO`.
pub(super) const IO: &str = "IO";
/// Resolved Python constructor vocabulary: `ItemsView`.
pub(super) const ITEMS_VIEW: &str = "ItemsView";
/// Resolved Python constructor vocabulary: `Iterable`.
pub(super) const ITERABLE: &str = "Iterable";
/// Resolved Python constructor vocabulary: `Iterator`.
pub(super) const ITERATOR: &str = "Iterator";
/// Resolved Python constructor vocabulary: `KeysView`.
pub(super) const KEYS_VIEW: &str = "KeysView";
/// Resolved Python constructor vocabulary: `List`.
pub(super) const LIST: &str = "List";
/// Resolved Python constructor vocabulary: `Mapping`.
pub(super) const MAPPING: &str = "Mapping";
/// Resolved Python constructor vocabulary: `Match`.
pub(super) const MATCH: &str = "Match";
/// Resolved Python constructor vocabulary: `MutableMapping`.
pub(super) const MUTABLE_MAPPING: &str = "MutableMapping";
/// Resolved Python constructor vocabulary: `MutableSequence`.
pub(super) const MUTABLE_SEQUENCE: &str = "MutableSequence";
/// Resolved Python constructor vocabulary: `MutableSet`.
pub(super) const MUTABLE_SET: &str = "MutableSet";
/// Resolved Python constructor vocabulary: `NotRequired`.
pub(super) const NOT_REQUIRED: &str = "NotRequired";
/// Resolved Python constructor vocabulary: `Optional`.
pub(super) const OPTIONAL: &str = "Optional";
/// Resolved Python constructor vocabulary: `OrderedDict`.
pub(super) const ORDERED_DICT: &str = "OrderedDict";
/// Resolved Python constructor vocabulary: `Pattern`.
pub(super) const PATTERN: &str = "Pattern";
/// Resolved Python constructor vocabulary: `Protocol`.
pub(super) const PROTOCOL: &str = "Protocol";
/// Resolved Python constructor vocabulary: `Required`.
pub(super) const REQUIRED: &str = "Required";
/// Resolved Python constructor vocabulary: `Reversible`.
pub(super) const REVERSIBLE: &str = "Reversible";
/// Resolved Python constructor vocabulary: `Sequence`.
pub(super) const SEQUENCE: &str = "Sequence";
/// Resolved Python constructor vocabulary: `Set`.
pub(super) const SET: &str = "Set";
/// Resolved Python constructor vocabulary: `Tuple`.
pub(super) const TUPLE: &str = "Tuple";
/// Resolved Python constructor vocabulary: `Type`.
pub(super) const TYPE: &str = "Type";
/// Resolved Python constructor vocabulary: `TypeGuard`.
pub(super) const TYPE_GUARD: &str = "TypeGuard";
/// Resolved Python constructor vocabulary: `TypeIs`.
pub(super) const TYPE_IS: &str = "TypeIs";
/// Resolved Python constructor vocabulary: `Union`.
pub(super) const UNION: &str = "Union";
/// Resolved Python constructor vocabulary: `Unpack`.
pub(super) const UNPACK: &str = "Unpack";
/// Resolved Python constructor vocabulary: `ValuesView`.
pub(super) const VALUES_VIEW: &str = "ValuesView";
/// Resolved Python constructor vocabulary: `collections.abc.`.
pub(super) const COLLECTIONS_ABSTRACT: &str = "collections.abc.";
/// Resolved Python constructor vocabulary: `MappingView`.
pub(super) const MAPPING_VIEW: &str = "MappingView";
/// Resolved Python constructor vocabulary: `collections.ChainMap`.
pub(super) const COLLECTIONS_CHAIN_MAP: &str = "collections.ChainMap";
/// Resolved Python constructor vocabulary: `collections.Counter`.
pub(super) const COLLECTIONS_COUNTER: &str = "collections.Counter";
/// Resolved Python constructor vocabulary: `collections.OrderedDict`.
pub(super) const COLLECTIONS_ORDERED_DICT: &str = "collections.OrderedDict";
/// Resolved Python constructor vocabulary: `collections.defaultdict`.
pub(super) const COLLECTIONS_DEFAULTDICT: &str = "collections.defaultdict";
/// Resolved Python constructor vocabulary: `collections.deque`.
pub(super) const COLLECTIONS_DEQUE: &str = "collections.deque";
/// Resolved Python constructor vocabulary: `contextlib.AbstractContextManager`.
pub(super) const CONTEXTLIB_ABSTRACT_CONTEXT_MANAGER: &str = "contextlib.AbstractContextManager";
/// Resolved Python constructor vocabulary: `contextlib.AbstractAsyncContextManager`.
pub(super) const CONTEXTLIB_ABSTRACT_ASYNC_CONTEXT_MANAGER: &str =
    "contextlib.AbstractAsyncContextManager";
/// Resolved Python constructor vocabulary: `re.Match`.
pub(super) const RE_MATCH: &str = "re.Match";
/// Resolved Python constructor vocabulary: `re.Pattern`.
pub(super) const RE_PATTERN: &str = "re.Pattern";
/// Resolved Python constructor vocabulary: `weakref.WeakKeyDictionary`.
pub(super) const WEAKREF_WEAK_KEY_DICTIONARY: &str = "weakref.WeakKeyDictionary";
/// Resolved Python constructor vocabulary: `weakref.WeakValueDictionary`.
pub(super) const WEAKREF_WEAK_VALUE_DICTIONARY: &str = "weakref.WeakValueDictionary";
/// Resolved Python constructor vocabulary: `weakref.WeakSet`.
pub(super) const WEAKREF_WEAK_SET: &str = "weakref.WeakSet";
