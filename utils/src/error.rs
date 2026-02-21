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

//! Defines the error types used throughout the `awesome-sails` workspace.
//!
//! This module provides a generic `Error` type wrapping a string message,
//! as well as specific error unit structs for common failure scenarios
//! such as invalid input, incorrect origin, bad value, and event emission failures.

use alloc::string::{String, ToString};
use core::fmt;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;

/// Represents a generic error type within the `awesome-sails` ecosystem.
///
/// This struct wraps a string message providing details about the error.
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
    /// Creates a new `Error` instance with the provided message.
    ///
    /// # Arguments
    ///
    /// * `message` - A value that can be converted into a `String`, describing the error.
    ///
    /// # Returns
    ///
    /// A new `Error` instance containing the converted message string.
    pub fn new(message: impl ToString) -> Self {
        Self(message.to_string())
    }
}

impl<E: core::error::Error> From<E> for Error {
    fn from(err: E) -> Self {
        Self::new(err)
    }
}

/// Indicates an incorrect input argument was provided.
///
/// This error is typically returned when an argument does not meet the expected criteria.
#[derive(Clone, Debug, Decode, Default, Encode, TypeInfo, thiserror::Error)]
#[codec(crate = parity_scale_codec)]
#[error("incorrect input argument")]
#[scale_info(crate = scale_info)]
pub struct BadInput;

/// Indicates an operation was attempted by an incorrect origin.
///
/// This error is typically returned when the message sender does not have the required permissions
/// or is not the expected entity.
#[derive(Clone, Debug, Decode, Default, Encode, TypeInfo, thiserror::Error)]
#[codec(crate = parity_scale_codec)]
#[error("incorrect message origin")]
#[scale_info(crate = scale_info)]
pub struct BadOrigin;

/// Indicates an incorrect value was attached to the message.
///
/// This error is typically returned when the transferred value is insufficient or invalid for the operation.
#[derive(Clone, Debug, Decode, Default, Encode, TypeInfo, thiserror::Error)]
#[codec(crate = parity_scale_codec)]
#[error("incorrect message value")]
#[scale_info(crate = scale_info)]
pub struct BadValue;

/// Indicates a failure occurred while attempting to emit an event.
///
/// This error is typically returned when the event emission mechanism encounters an issue.
#[derive(Clone, Debug, Decode, Default, Encode, TypeInfo, thiserror::Error)]
#[codec(crate = parity_scale_codec)]
#[error("emit event error")]
#[scale_info(crate = scale_info)]
pub struct EmitError;
