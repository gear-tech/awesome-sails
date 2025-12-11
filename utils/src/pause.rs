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

//! Awesome pausable storage primitive.

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

/// Wrapper for Storage trait implementor in order to provide pause functionality.
pub struct Pausable<S: StorageMut, P: InfallibleStorage<Item = Pause>> {
    storage: S,
    pause: P,
}

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
    /// Creates a new `Pausable` instance linked to a `Pause` instance.
    pub fn new(pause: P, storage: S) -> Self {
        Self { pause, storage }
    }

    /// Creates a new `Pausable` instance with a default storage value.
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

pub trait PausableStorage: StorageMut {
    /// Returns bool indicating if pause is on.
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

/// Struct representing a pause switch.
///
/// This struct is used to create a pausable storage instance.
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Pause(Cell<bool>);

pub type PauseRef<'a> = &'a Pause;

impl Pause {
    /// Creates a new `Pause` instance.
    pub fn new(paused: bool) -> Self {
        Self(Cell::new(paused))
    }

    /// Switches pause on.
    ///
    /// Returns bool indicating if state was changed.
    pub fn pause(&self) -> bool {
        !self.0.replace(true)
    }

    /// Switches pause off.
    ///
    /// Returns bool indicating if state was changed.
    pub fn resume(&self) -> bool {
        self.0.replace(false)
    }

    /// Returns bool indicating if pause is on.
    pub fn is_paused(&self) -> bool {
        self.0.get()
    }
}

/// Error type for the `Pausable` storage.
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Decode, Encode, TypeInfo, thiserror::Error,
)]
#[codec(crate = parity_scale_codec)]
#[scale_info(crate = scale_info)]
pub enum PausableError<E: error::Error> {
    /// Error indicating that the storage is paused.
    #[error("storage is paused")]
    Paused,
    /// Error indicating inner storage error.
    #[error("storage error: {0}")]
    Storage(#[from] E),
}

#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Decode, Encode, TypeInfo, thiserror::Error,
)]
#[codec(crate = parity_scale_codec)]
#[error("enabled pause error")]
#[scale_info(crate = scale_info)]
pub struct PausedError;

#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Decode, Encode, TypeInfo, thiserror::Error,
)]
#[codec(crate = parity_scale_codec)]
#[error("disabled pause error")]
#[scale_info(crate = scale_info)]
pub struct UnpausedError;
