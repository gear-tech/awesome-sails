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
