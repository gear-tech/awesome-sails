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

use crate::{ensure, storage::Storage};
use core::{
    cell::{Ref, RefCell, RefMut},
    error,
    ops::Deref,
};
use sails_rs::{Decode, Encode, TypeInfo, rc::Rc};

/// Wrapper for Storage trait implementor in order to provide pause functionality.
pub struct Pausable<S: Storage> {
    pause: Pause,
    storage: S,
}

impl<S: Storage> Pausable<S> {
    /// Creates a new `Pausable` instance linked to a `Pause` instance.
    pub fn new(pause: &Pause, storage: S) -> Self {
        Self {
            pause: pause.clone(),
            storage,
        }
    }

    /// Creates a new `Pausable` instance with a default storage value.
    pub fn default(pause: &Pause) -> Self
    where
        S: Default,
    {
        Self::new(pause, Default::default())
    }
}

impl<S: Storage> Storage for Pausable<S>
where
    S::Error: 'static,
{
    type Item = S::Item;
    type Error = PausableError<S::Error>;

    fn get(&self) -> Result<Ref<'_, Self::Item>, Self::Error> {
        self.storage.get().map_err(Into::into)
    }

    fn get_mut(&self) -> Result<RefMut<'_, Self::Item>, Self::Error> {
        ensure!(!self.pause.is_paused(), PausableError::Paused);

        self.storage.get_mut().map_err(Into::into)
    }

    fn replace(&self, value: Self::Item) -> Result<Self::Item, Self::Error> {
        ensure!(!self.pause.is_paused(), PausableError::Paused);

        self.storage.replace(value).map_err(Into::into)
    }

    fn replace_with(
        &self,
        f: impl FnOnce(&mut Self::Item) -> Self::Item,
    ) -> Result<Self::Item, Self::Error> {
        ensure!(!self.pause.is_paused(), PausableError::Paused);

        self.storage.replace_with(f).map_err(Into::into)
    }
}

// TODO: once supported &mut program in exposures - change here mutability.
/// Struct representing a pause switch.
///
/// This struct is used to create a pausable storage instance.
#[derive(Clone, Debug, Default)]
pub struct Pause(Rc<RefCell<bool>>);

impl Pause {
    /// Creates a new `Pause` instance.
    pub fn new(paused: bool) -> Self {
        Self(Rc::new(RefCell::new(paused)))
    }

    /// Switches pause on.
    ///
    /// Returns bool indicating if state was changed.
    pub fn pause(&self) -> bool {
        !self.0.deref().replace(true)
    }

    /// Switches pause off.
    ///
    /// Returns bool indicating if state was changed.
    pub fn resume(&self) -> bool {
        self.0.deref().replace(false)
    }

    /// Returns bool indicating if pause is on.
    pub fn is_paused(&self) -> bool {
        *(self.0.deref().borrow())
    }
}

/// Error type for the `Pausable` storage.
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Decode, Encode, TypeInfo, thiserror::Error,
)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
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
#[codec(crate = sails_rs::scale_codec)]
#[error("enabled pause error")]
#[scale_info(crate = sails_rs::scale_info)]
pub struct PausedError;

#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Decode, Encode, TypeInfo, thiserror::Error,
)]
#[codec(crate = sails_rs::scale_codec)]
#[error("disabled pause error")]
#[scale_info(crate = sails_rs::scale_info)]
pub struct UnpausedError;
