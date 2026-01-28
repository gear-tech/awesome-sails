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

//! The `awesome-sails` meta-crate.
//!
//! This crate bundles various services and utilities for building dApps on the
//! Gear Protocol using the Sails Framework. It allows selectively enabling features
//! to include only the necessary components.

#![no_std]

/// Core Vara Fungible Token (VFT) implementation.
#[cfg(feature = "vft")]
pub use awesome_sails_vft as vft;

/// Shared utilities for VFT implementation (allowances, balances).
#[cfg(feature = "vft-utils")]
pub use awesome_sails_vft_utils as vft_utils;

/// Administrative service for VFT (mint, burn, pause).
#[cfg(feature = "vft-admin")]
pub use awesome_sails_vft_admin as vft_admin;

/// Extended functionality for VFT (transfer all, cleanup, enumeration).
#[cfg(feature = "vft-extension")]
pub use awesome_sails_vft_extension as vft_extension;

/// Metadata service for VFT (name, symbol, decimals).
#[cfg(feature = "vft-metadata")]
pub use awesome_sails_vft_metadata as vft_metadata;

/// Native value exchange service for VFT.
#[cfg(feature = "vft-native-exchange")]
pub use awesome_sails_vft_native_exchange as vft_native_exchange;

/// Administrative service for Native Exchange (burn from, failed mint handling).
#[cfg(feature = "vft-native-exchange-admin")]
pub use awesome_sails_vft_native_exchange_admin as vft_native_exchange_admin;

/// Role-Based Access Control (RBAC) service.
#[cfg(feature = "access-control")]
pub use awesome_sails_access_control as access_control;
