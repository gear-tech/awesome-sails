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

//! Awesome VFT-Metadata service.
//!
//! This service provides access to the metadata (name, symbol, decimals) of a
//! Volatile Fungible Token (VFT), following the ERC-20 metadata standard.
//! It is typically designed to be immutable after deployment, though the storage backend
//! determines mutability.

#![no_std]

use awesome_sails_utils::storage::InfallibleStorage;
use core::ops::Deref;
use sails_rs::prelude::*;

/// The VFT Metadata service struct.
///
/// Wraps the metadata storage to provide read access via the service interface.
pub struct VftMetadata<M> {
    // Metadata storage.
    metadata: M,
}

impl<M> VftMetadata<M> {
    /// Creates a new instance of the VFT Metadata service.
    ///
    /// # Arguments
    ///
    /// * `metadata` - The storage backend containing the [`Metadata`].
    pub fn new(metadata: M) -> Self {
        Self { metadata }
    }
}

impl<M: InfallibleStorage<Item = Metadata>> VftMetadata<M> {
    /// Retrieves a reference to the inner metadata object from storage.
    ///
    /// # Returns
    ///
    /// A dereferenceable wrapper around [`Metadata`].
    pub fn metadata(&self) -> impl Deref<Target = Metadata> {
        self.metadata.get()
    }
}

#[service]
impl<M: InfallibleStorage<Item = Metadata>> VftMetadata<M> {
    /// Returns the name of the VFT.
    ///
    /// # Returns
    ///
    /// The name as a `String`.
    #[export]
    pub fn name(&self) -> String {
        self.metadata().name().into()
    }

    /// Returns the symbol of the VFT.
    ///
    /// # Returns
    ///
    /// The symbol as a `String`.
    #[export]
    pub fn symbol(&self) -> String {
        self.metadata().symbol().into()
    }

    /// Returns the number of decimals used by the VFT.
    ///
    /// # Returns
    ///
    /// The decimals as a `u8`.
    #[export]
    pub fn decimals(&self) -> u8 {
        self.metadata().decimals()
    }
}

/// Represents the metadata of a VFT: name, symbol, and decimals.
#[derive(Debug, Clone, Encode, Decode, TypeInfo, PartialEq, Eq, PartialOrd, Ord)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub struct Metadata {
    name: String,
    symbol: String,
    decimals: u8,
}

impl Metadata {
    /// Creates a new metadata instance.
    ///
    /// # Arguments
    ///
    /// * `name` - The full name of the token.
    /// * `symbol` - The abbreviated symbol of the token.
    /// * `decimals` - The precision of the token (e.g., 18).
    pub const fn new(name: String, symbol: String, decimals: u8) -> Self {
        Self {
            name,
            symbol,
            decimals,
        }
    }

    /// Returns the name of the VFT.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the symbol of the VFT.
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    /// Returns the number of decimals of the VFT.
    pub fn decimals(&self) -> u8 {
        self.decimals
    }
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            name: "Unit".into(),
            symbol: "UNIT".into(),
            decimals: 12,
        }
    }
}
