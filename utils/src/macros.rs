// This file is part of Gear.

// Copyright (C) 2025 Gear Technologies Inc.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Awesome macros definition module.

#[macro_export]
macro_rules! assert_ok {
    ( $x:expr, $y: expr $(,)? ) => {
        assert_eq!($x.expect("Ran into `Err` value"), $y);
    };
}

#[macro_export]
macro_rules! assert_err {
    ( $x:expr, $y: expr $(,)? ) => {
        assert_eq!($x.err().expect("Ran into `Ok` value"), $y);
    };
}

#[macro_export]
macro_rules! bail {
    ($err: expr) => {
        return Err($err.into());
    };
}

#[macro_export]
macro_rules! ensure {
    ($cond: expr, $err: literal) => {
        $crate::ensure!($cond, $crate::error::Error::new($err));
    };

    ($cond: expr, $err:expr) => {
        if !$cond {
            return Err($err.into());
        }
    };
}

#[macro_export]
macro_rules! ok_if {
    ($cond: expr) => {
        $crate::ok_if!($cond, ());
    };
    ($cond: expr, $ok: expr) => {
        if $cond {
            return Ok($ok.into());
        }
    };
}

#[macro_export]
macro_rules! unwrap_infallible {
    ($res: expr) => {
        match $res {
            Ok(r) => r,
            Err(e) => match e {}, // Unreachable pattern
        }
    };
}

#[macro_export]
macro_rules! impl_non_zero_conversion {
    ($($name: ident),*) => {
        $(
            impl TryFrom<$name> for $crate::math::NonZero<$name> {
                type Error = $crate::math::ZeroError;

                fn try_from(value: $name) -> Result<Self, Self::Error> {
                    $crate::math::NonZero::try_new(value)
                }
            }

                        impl From<$crate::math::NonZero<$name>> for $name {
                            fn from(value: $crate::math::NonZero<$name>) -> Self {
                                value.into_inner()
                            }
                        }
                    )*
                };
            }

/// Verifies that given wrapper is small enough (< 16 bytes)
/// to be used with default math operations.
///
/// Usage: `impl_math_wrapper!(WrapperName, LeBytes<AMOUNT_OF_BYTES>);`
///
/// Requires:
/// - `PartialEq`
///
/// Derives:
/// - `TryFrom<u128>`
/// - `TryFrom<U256>`
/// - `Into<u128>`
/// - `Into<U256>`
/// - `TryInto<NonZero<Self>>`
/// - `From<NonZero<Self>>`
/// - `Max`
/// - `Min`
/// - `One`
/// - `Zero`
/// - `CheckedMath`
#[macro_export]
macro_rules! impl_math_wrapper {
    ($wrapper:ident, LeBytes<$n:literal>) => {
        const _: () = assert!(
            $n < 16,
            "should only be used for small le bytes wrappers (< 16 bytes)"
        );

        const _: $wrapper = $wrapper(<$crate::math::LeBytes<$n> as $crate::math::Zero>::ZERO);

        $crate::impl_non_zero_conversion!($wrapper);

        impl $crate::math::Max for $wrapper {
            const MAX: Self = Self(<$crate::math::LeBytes<$n>>::MAX);
        }
        impl $crate::math::Min for $wrapper {
            const MIN: Self = Self(<$crate::math::LeBytes<$n>>::MIN);
        }
        impl $crate::math::Zero for $wrapper {
            const ZERO: Self = Self(<$crate::math::LeBytes<$n>>::ZERO);
        }
        impl $crate::math::One for $wrapper {
            const ONE: Self = Self(<$crate::math::LeBytes<$n>>::ONE);
        }

        impl $crate::math::CheckedMath for $wrapper {
            fn checked_add(self, rhs: Self) -> Option<Self> {
                self.0.checked_add(rhs.0).map(Self)
            }
            fn checked_sub(self, rhs: Self) -> Option<Self> {
                self.0.checked_sub(rhs.0).map(Self)
            }
        }

        impl PartialEq<$crate::math::NonZero<$wrapper>> for $wrapper {
            fn eq(&self, other: &$crate::math::NonZero<$wrapper>) -> bool {
                self.eq(&other.0)
            }
        }
        impl PartialOrd<$crate::math::NonZero<$wrapper>> for $wrapper {
            fn partial_cmp(
                &self,
                other: &$crate::math::NonZero<$wrapper>,
            ) -> Option<core::cmp::Ordering> {
                self.partial_cmp(&other.0)
            }
        }

        impl PartialEq<$crate::math::LeBytes<$n>> for $wrapper {
            fn eq(&self, other: &$crate::math::LeBytes<$n>) -> bool {
                self.0 == *other
            }
        }

        impl PartialOrd<$crate::math::LeBytes<$n>> for $wrapper {
            fn partial_cmp(
                &self,
                other: &$crate::math::LeBytes<$n>,
            ) -> Option<core::cmp::Ordering> {
                self.0.partial_cmp(other)
            }
        }

        impl TryFrom<$crate::math::U256> for $wrapper {
            type Error = $crate::math::OverflowError;
            fn try_from(value: $crate::math::U256) -> Result<Self, Self::Error> {
                let inner = <$crate::math::LeBytes<$n>>::try_from(value)
                    .map_err(|_| $crate::math::OverflowError)?;
                Ok(Self(inner))
            }
        }

        impl TryFrom<u128> for $wrapper {
            type Error = $crate::math::OverflowError;
            fn try_from(value: u128) -> Result<Self, Self::Error> {
                let inner = <$crate::math::LeBytes<$n>>::try_from(value)
                    .map_err(|_| $crate::math::OverflowError)?;
                Ok(Self(inner))
            }
        }

        impl From<$wrapper> for u128 {
            fn from(value: $wrapper) -> u128 {
                match value.0.try_into().map_err(|_| unreachable!()) {
                    Ok(v) => v,
                    Err(inf) => inf,
                }
            }
        }

        impl From<$wrapper> for $crate::math::U256 {
            fn from(value: $wrapper) -> $crate::math::U256 {
                match value.0.try_into().map_err(|_| unreachable!()) {
                    Ok(v) => v,
                    Err(inf) => inf,
                }
            }
        }

        impl From<$crate::math::LeBytes<$n>> for $wrapper {
            fn from(v: $crate::math::LeBytes<$n>) -> Self {
                Self(v)
            }
        }

        impl From<$wrapper> for $crate::math::LeBytes<$n> {
            fn from(v: $wrapper) -> Self {
                v.0
            }
        }
    };
}
