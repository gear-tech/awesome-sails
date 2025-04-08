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

// TODO: push sails changes.
//! Current awesome impl of event emitters. To be replaced with proper sails changes.

use sails_rs::{Decode, Encode, TypeInfo};

/// Trait for emitting events.
pub trait Emitter {
    /// The type of event that can be emitted.
    type Event: Encode + TypeInfo;

    /// Function to derive.
    fn notify(&mut self, event: Self::Event) -> Result<(), sails_rs::errors::Error>;

    /// Emits an event and returns a result indicating success or failure.
    fn emit(&mut self, event: Self::Event) -> Result<(), EmitError> {
        self.notify(event).map_err(|_| EmitError)
    }

    /// Emits an event and panics if the emit fails.
    fn emit_or_panic(&mut self, event: Self::Event) {
        self.emit(event).expect("failed to emit event");
    }
}

/// Error type for inability to emit event.
#[derive(Clone, Debug, Decode, Default, Encode, TypeInfo, thiserror::Error)]
#[codec(crate = sails_rs::scale_codec)]
#[error("emit event error")]
#[scale_info(crate = sails_rs::scale_info)]
pub struct EmitError;
