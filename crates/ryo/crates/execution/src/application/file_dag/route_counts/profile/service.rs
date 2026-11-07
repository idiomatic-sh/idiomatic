//! Thread-local storage of the active file profile.
use std::cell::RefCell;

use super::super::types::RouteState;

thread_local! { pub(in super::super) static STATE: RefCell<Option<RouteState>> = const { RefCell::new(None) }; }
