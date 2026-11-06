//! Checked arithmetic of the ledger's counts; an overflow is the caller's refusal.
use super::types::{ByteCount, HoldCount, UnitCount};
use std::ops::{AddAssign, SubAssign};

macro_rules! counted {
    ($($count:ident),+) => {$(
        impl $count {
            pub(in crate::application::file_dag) fn checked_add(self, other: Self) -> Option<Self> {
                self.0.checked_add(other.0).map(Self)
            }
        }
        impl AddAssign for $count {
            fn add_assign(&mut self, other: Self) {
                self.0 += other.0;
            }
        }
        impl SubAssign for $count {
            fn sub_assign(&mut self, other: Self) {
                self.0 -= other.0;
            }
        }
    )+};
}

counted!(ByteCount, UnitCount, HoldCount);

impl ByteCount {
    pub(in crate::application::file_dag) fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }
}
