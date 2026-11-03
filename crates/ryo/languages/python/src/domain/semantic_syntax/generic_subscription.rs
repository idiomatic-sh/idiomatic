//! Resolved Python type constructors; spelling alone never establishes a type context.

/// Whether subscription operands of this resolved standard-library object are types.
/// Literal and Annotated have distinct value-operand rules handled by the caller.
pub(super) fn is_type_constructor(path: &str) -> bool {
    if let Some(name) = path.strip_prefix("builtins.") {
        return matches!(
            name,
            "dict" | "frozenset" | "list" | "set" | "tuple" | "type"
        );
    }
    if let Some(name) = path
        .strip_prefix("typing.")
        .or_else(|| path.strip_prefix("typing_extensions."))
    {
        return matches!(
            name,
            "AbstractSet"
                | "AsyncContextManager"
                | "AsyncGenerator"
                | "AsyncIterable"
                | "AsyncIterator"
                | "Awaitable"
                | "Callable"
                | "ChainMap"
                | "ClassVar"
                | "Collection"
                | "Concatenate"
                | "Container"
                | "ContextManager"
                | "Coroutine"
                | "Counter"
                | "DefaultDict"
                | "Deque"
                | "Dict"
                | "Final"
                | "FrozenSet"
                | "Generator"
                | "Generic"
                | "IO"
                | "ItemsView"
                | "Iterable"
                | "Iterator"
                | "KeysView"
                | "List"
                | "Mapping"
                | "Match"
                | "MutableMapping"
                | "MutableSequence"
                | "MutableSet"
                | "NotRequired"
                | "Optional"
                | "OrderedDict"
                | "Pattern"
                | "Protocol"
                | "Required"
                | "Reversible"
                | "Sequence"
                | "Set"
                | "Tuple"
                | "Type"
                | "TypeGuard"
                | "TypeIs"
                | "Union"
                | "Unpack"
                | "ValuesView"
        );
    }
    if let Some(name) = path.strip_prefix("collections.abc.") {
        return matches!(
            name,
            "AsyncGenerator"
                | "AsyncIterable"
                | "AsyncIterator"
                | "Awaitable"
                | "Callable"
                | "Collection"
                | "Container"
                | "Coroutine"
                | "Generator"
                | "ItemsView"
                | "Iterable"
                | "Iterator"
                | "KeysView"
                | "Mapping"
                | "MappingView"
                | "MutableMapping"
                | "MutableSequence"
                | "MutableSet"
                | "Reversible"
                | "Sequence"
                | "Set"
                | "ValuesView"
        );
    }
    matches!(
        path,
        "collections.ChainMap"
            | "collections.Counter"
            | "collections.OrderedDict"
            | "collections.defaultdict"
            | "collections.deque"
            | "contextlib.AbstractContextManager"
            | "contextlib.AbstractAsyncContextManager"
            | "re.Match"
            | "re.Pattern"
            | "weakref.WeakKeyDictionary"
            | "weakref.WeakValueDictionary"
            | "weakref.WeakSet"
    )
}
