//! Whether the subscription operands of a resolved standard-library object are types.

use super::super::constants::{
    ABSTRACT_SET, ASYNC_CONTEXT_MANAGER, ASYNC_GENERATOR, ASYNC_ITERABLE, ASYNC_ITERATOR,
    AWAITABLE, BUILTIN_DICT, BUILTIN_FROZENSET, BUILTIN_LIST, BUILTIN_SET, BUILTIN_TUPLE,
    BUILTIN_TYPE, BUILTINS, CALLABLE, CHAIN_MAP, CLASS_VAR, COLLECTION, COLLECTIONS_ABSTRACT,
    COLLECTIONS_CHAIN_MAP, COLLECTIONS_COUNTER, COLLECTIONS_DEFAULTDICT, COLLECTIONS_DEQUE,
    COLLECTIONS_ORDERED_DICT, CONCATENATE, CONTAINER, CONTEXT_MANAGER,
    CONTEXTLIB_ABSTRACT_ASYNC_CONTEXT_MANAGER, CONTEXTLIB_ABSTRACT_CONTEXT_MANAGER, COROUTINE,
    COUNTER, DEFAULT_DICT, DEQUE, DICT, FINAL, FROZEN_SET, GENERATOR, GENERIC, IO, ITEMS_VIEW,
    ITERABLE, ITERATOR, KEYS_VIEW, LIST, MAPPING, MAPPING_VIEW, MATCH, MUTABLE_MAPPING,
    MUTABLE_SEQUENCE, MUTABLE_SET, NOT_REQUIRED, OPTIONAL, ORDERED_DICT, PATTERN, PROTOCOL,
    RE_MATCH, RE_PATTERN, REQUIRED, REVERSIBLE, SEQUENCE, SET, TUPLE, TYPE, TYPE_GUARD, TYPE_IS,
    TYPING, TYPING_EXTENSIONS, UNION, UNPACK, VALUES_VIEW, WEAKREF_WEAK_KEY_DICTIONARY,
    WEAKREF_WEAK_SET, WEAKREF_WEAK_VALUE_DICTIONARY,
};
/// Whether subscription operands of this resolved standard-library object are types.
/// Literal and Annotated have distinct value-operand rules handled by the caller.
pub(in super::super) fn operands(
    path: ryo::languages::NameText<'_>,
) -> super::super::types::TypeContext {
    use super::super::types::TypeContext;
    let path = path.0;
    if let Some(name) = path.strip_prefix(BUILTINS) {
        return if matches!(
            name,
            BUILTIN_DICT
                | BUILTIN_FROZENSET
                | BUILTIN_LIST
                | BUILTIN_SET
                | BUILTIN_TUPLE
                | BUILTIN_TYPE
        ) {
            TypeContext::Type
        } else {
            TypeContext::Value
        };
    }
    if let Some(name) = path
        .strip_prefix(TYPING)
        .or_else(|| path.strip_prefix(TYPING_EXTENSIONS))
    {
        return if matches!(
            name,
            ABSTRACT_SET
                | ASYNC_CONTEXT_MANAGER
                | ASYNC_GENERATOR
                | ASYNC_ITERABLE
                | ASYNC_ITERATOR
                | AWAITABLE
                | CALLABLE
                | CHAIN_MAP
                | CLASS_VAR
                | COLLECTION
                | CONCATENATE
                | CONTAINER
                | CONTEXT_MANAGER
                | COROUTINE
                | COUNTER
                | DEFAULT_DICT
                | DEQUE
                | DICT
                | FINAL
                | FROZEN_SET
                | GENERATOR
                | GENERIC
                | IO
                | ITEMS_VIEW
                | ITERABLE
                | ITERATOR
                | KEYS_VIEW
                | LIST
                | MAPPING
                | MATCH
                | MUTABLE_MAPPING
                | MUTABLE_SEQUENCE
                | MUTABLE_SET
                | NOT_REQUIRED
                | OPTIONAL
                | ORDERED_DICT
                | PATTERN
                | PROTOCOL
                | REQUIRED
                | REVERSIBLE
                | SEQUENCE
                | SET
                | TUPLE
                | TYPE
                | TYPE_GUARD
                | TYPE_IS
                | UNION
                | UNPACK
                | VALUES_VIEW
        ) {
            TypeContext::Type
        } else {
            TypeContext::Value
        };
    }
    if let Some(name) = path.strip_prefix(COLLECTIONS_ABSTRACT) {
        return if matches!(
            name,
            ASYNC_GENERATOR
                | ASYNC_ITERABLE
                | ASYNC_ITERATOR
                | AWAITABLE
                | CALLABLE
                | COLLECTION
                | CONTAINER
                | COROUTINE
                | GENERATOR
                | ITEMS_VIEW
                | ITERABLE
                | ITERATOR
                | KEYS_VIEW
                | MAPPING
                | MAPPING_VIEW
                | MUTABLE_MAPPING
                | MUTABLE_SEQUENCE
                | MUTABLE_SET
                | REVERSIBLE
                | SEQUENCE
                | SET
                | VALUES_VIEW
        ) {
            TypeContext::Type
        } else {
            TypeContext::Value
        };
    }
    if matches!(
        path,
        COLLECTIONS_CHAIN_MAP
            | COLLECTIONS_COUNTER
            | COLLECTIONS_ORDERED_DICT
            | COLLECTIONS_DEFAULTDICT
            | COLLECTIONS_DEQUE
            | CONTEXTLIB_ABSTRACT_CONTEXT_MANAGER
            | CONTEXTLIB_ABSTRACT_ASYNC_CONTEXT_MANAGER
            | RE_MATCH
            | RE_PATTERN
            | WEAKREF_WEAK_KEY_DICTIONARY
            | WEAKREF_WEAK_VALUE_DICTIONARY
            | WEAKREF_WEAK_SET
    ) {
        TypeContext::Type
    } else {
        TypeContext::Value
    }
}
