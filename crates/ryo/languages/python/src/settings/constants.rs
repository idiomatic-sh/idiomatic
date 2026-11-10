//! The identities of the parser settings.

use ryo::languages::ParseConfigurationIdentity;

/// The identity of the `3.7` target.
pub(super) const IDENTITY_PY37: ParseConfigurationIdentity<'static> =
    ParseConfigurationIdentity("python-target=3.7");

/// The identity of the `3.8` target.
pub(super) const IDENTITY_PY38: ParseConfigurationIdentity<'static> =
    ParseConfigurationIdentity("python-target=3.8");

/// The identity of the `3.9` target.
pub(super) const IDENTITY_PY39: ParseConfigurationIdentity<'static> =
    ParseConfigurationIdentity("python-target=3.9");

/// The identity of the `3.10` target.
pub(super) const IDENTITY_PY310: ParseConfigurationIdentity<'static> =
    ParseConfigurationIdentity("python-target=3.10");

/// The identity of the `3.11` target.
pub(super) const IDENTITY_PY311: ParseConfigurationIdentity<'static> =
    ParseConfigurationIdentity("python-target=3.11");

/// The identity of the `3.12` target.
pub(super) const IDENTITY_PY312: ParseConfigurationIdentity<'static> =
    ParseConfigurationIdentity("python-target=3.12");

/// The identity of the `3.13` target.
pub(super) const IDENTITY_PY313: ParseConfigurationIdentity<'static> =
    ParseConfigurationIdentity("python-target=3.13");

/// The identity of the `3.14` target.
pub(super) const IDENTITY_PY314: ParseConfigurationIdentity<'static> =
    ParseConfigurationIdentity("python-target=3.14");

/// The identity of the `3.15` target.
pub(super) const IDENTITY_PY315: ParseConfigurationIdentity<'static> =
    ParseConfigurationIdentity("python-target=3.15");

/// The identity of the `latest` target.
pub(super) const IDENTITY_LATEST: ParseConfigurationIdentity<'static> =
    ParseConfigurationIdentity("python-target=latest");
