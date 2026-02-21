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

//! Implements a pausable storage wrapper.
//!
//! This module provides a mechanism to wrap any storage implementation and add a
//! pause/resume functionality. This is useful for emergency stops or maintenance modes
//! in smart contracts.

use crate::{
    ensure,
    storage::{InfallibleStorage, Storage, StorageMut, StorageRefCell},
};
use core::{
    cell::Cell,
    error,
    ops::{Deref, DerefMut},
};
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;

/// A wrapper around a storage type that adds pause functionality.
///
/// When paused, mutating operations on the storage will fail with `PausableError::Paused`.
/// Read-only operations are typically still allowed.
pub struct Pausable<S: StorageMut, P: InfallibleStorage<Item = Pause>> {
    storage: S,
    pause: P,
}

/// A convenience type alias for a `Pausable` utilizing reference cells for storage and pause state.
pub type PausableRef<'a, T> = Pausable<StorageRefCell<'a, T>, PauseRef<'a>>;

impl<S, P> Clone for Pausable<S, P>
where
    S: StorageMut + Clone,
    P: InfallibleStorage<Item = Pause> + Clone,
{
    fn clone(&self) -> Self {
        Self {
            storage: self.storage.clone(),
            pause: self.pause.clone(),
        }
    }
}

impl<S: StorageMut, P: InfallibleStorage<Item = Pause>> Pausable<S, P> {
    /// Creates a new `Pausable` wrapper.
    ///
    /// # Arguments
    ///
    /// * `pause` - The storage component holding the pause state.
    /// * `storage` - The underlying storage component to be wrapped.
    pub fn new(pause: P, storage: S) -> Self {
        Self { pause, storage }
    }

    /// Creates a new `Pausable` wrapper with default storage.
    ///
    /// # Arguments
    ///
    /// * `pause` - The storage component holding the pause state.
    pub fn default(pause: P) -> Self
    where
        S: Default,
    {
        Self::new(pause, Default::default())
    }
}

impl<S: StorageMut, P: InfallibleStorage<Item = Pause>> Storage for Pausable<S, P>
where
    S::Error: 'static,
{
    type Item = S::Item;
    type Error = PausableError<S::Error>;

    fn get(&self) -> Result<impl Deref<Target = Self::Item>, Self::Error> {
        self.storage.get().map_err(Into::into)
    }
}

impl<S: StorageMut, P: InfallibleStorage<Item = Pause>> StorageMut for Pausable<S, P>
where
    S::Error: 'static,
{
    fn get_mut(&mut self) -> Result<impl DerefMut<Target = Self::Item>, Self::Error> {
        ensure!(!self.pause.get().is_paused(), PausableError::Paused);

        self.storage.get_mut().map_err(Into::into)
    }

    fn replace(&mut self, value: Self::Item) -> Result<Self::Item, Self::Error>
    where
        S::Item: Sized,
    {
        ensure!(!self.pause.get().is_paused(), PausableError::Paused);

        self.storage.replace(value).map_err(Into::into)
    }

    fn replace_with(
        &mut self,
        f: impl FnOnce(&mut Self::Item) -> Self::Item,
    ) -> Result<Self::Item, Self::Error>
    where
        S::Item: Sized,
    {
        ensure!(!self.pause.get().is_paused(), PausableError::Paused);

        self.storage.replace_with(f).map_err(Into::into)
    }
}

/// A trait for storage types that support checking their pause state.
pub trait PausableStorage: StorageMut {
    /// Returns `true` if the storage is currently paused.
    fn is_paused(&self) -> bool;
}

impl<S, P> PausableStorage for Pausable<S, P>
where
    S: StorageMut,
    S::Error: 'static,
    P: InfallibleStorage<Item = Pause>,
{
    fn is_paused(&self) -> bool {
        self.pause.get().is_paused()
    }
}

/// A type representing a simple boolean pause switch.
///
/// Wraps a `Cell<bool>` to allow interior mutability for the pause state.
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Pause(Cell<bool>);

/// A type alias for a reference to a `Pause` instance.
pub type PauseRef<'a> = &'a Pause;

impl Pause {
    /// Creates a new `Pause` instance.
    ///
    /// # Arguments
    ///
    /// * `paused` - The initial state (true for paused, false for unpaused).
    pub fn new(paused: bool) -> Self {
        Self(Cell::new(paused))
    }

    /// Sets the state to paused.
    ///
    /// # Returns
    ///
    /// `true` if the state was changed (i.e., it was previously not paused).
    pub fn pause(&self) -> bool {
        !self.0.replace(true)
    }

    /// Sets the state to unpaused.
    ///
    /// # Returns
    ///
    /// `true` if the state was changed (i.e., it was previously paused).
    pub fn resume(&self) -> bool {
        self.0.replace(false)
    }

    /// Checks if the current state is paused.
    ///
    /// # Returns
    ///
    /// `true` if currently paused, `false` otherwise.
    pub fn is_paused(&self) -> bool {
        self.0.get()
    }
}

/// Error type for operations on pausable storage.
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Decode, Encode, TypeInfo, thiserror::Error,
)]
#[codec(crate = parity_scale_codec)]
#[scale_info(crate = scale_info)]
pub enum PausableError<E: error::Error> {
    /// The operation failed because the storage is paused.
    #[error("storage is paused")]
    Paused,
    /// An error occurred in the underlying storage.
    #[error("storage error: {0}")]
    Storage(#[from] E),
}

/// Error indicating that an operation required the system to be unpaused, but it was paused.
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Decode, Encode, TypeInfo, thiserror::Error,
)]
#[codec(crate = parity_scale_codec)]
#[error("enabled pause error")]
#[scale_info(crate = scale_info)]
pub struct PausedError;

/// Error indicating that an operation required the system to be paused, but it was unpaused.
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Decode, Encode, TypeInfo, thiserror::Error,
)]
#[codec(crate = parity_scale_codec)]
#[error("disabled pause error")]
#[scale_info(crate = scale_info)]
pub struct UnpausedError;
