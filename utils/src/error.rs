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

//! Awesome errors definition module.

use alloc::string::{String, ToString};
use core::fmt;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;

/// Error type for the `awesome-sails` library.
#[derive(Clone, Decode, Encode, TypeInfo, derive_more::Display)]
#[codec(crate = parity_scale_codec)]
#[scale_info(crate = scale_info)]
#[display("{}", _0)]
pub struct Error(String);

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error {
    /// Creates a new [`Self`] instance with the given message.
    pub fn new(message: impl ToString) -> Self {
        Self(message.to_string())
    }
}

impl<E: core::error::Error> From<E> for Error {
    fn from(err: E) -> Self {
        Self::new(err)
    }
}

/// Arbitrary error type for incorrect input argument.
#[derive(Clone, Debug, Decode, Default, Encode, TypeInfo, thiserror::Error)]
#[codec(crate = parity_scale_codec)]
#[error("incorrect input argument")]
#[scale_info(crate = scale_info)]
pub struct BadInput;

/// Arbitrary error type for incorrect origin.
#[derive(Clone, Debug, Decode, Default, Encode, TypeInfo, thiserror::Error)]
#[codec(crate = parity_scale_codec)]
#[error("incorrect message origin")]
#[scale_info(crate = scale_info)]
pub struct BadOrigin;

/// Arbitrary error type for incorrect (e.g. insufficient) value applied to the message.
#[derive(Clone, Debug, Decode, Default, Encode, TypeInfo, thiserror::Error)]
#[codec(crate = parity_scale_codec)]
#[error("incorrect message value")]
#[scale_info(crate = scale_info)]
pub struct BadValue;

/// Error type for inability to emit event.
#[derive(Clone, Debug, Decode, Default, Encode, TypeInfo, thiserror::Error)]
#[codec(crate = parity_scale_codec)]
#[error("emit event error")]
#[scale_info(crate = scale_info)]
pub struct EmitError;
