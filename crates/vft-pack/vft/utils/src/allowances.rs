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

//! Module for allowances-related VFT logic.

use crate::Allowance;
use awesome_sails_utils::{
    map::{ShardedMap, ShardedMapError},
    math::{Math, MathError, NonZero, UnderflowError, Zero},
    ok_if, unwrap_infallible,
};
use core::ops::Deref;
use sails_rs::{ActorId, Decode, Encode, TypeInfo, vec, vec::Vec};

pub type AllowancesKey = (NonZero<ActorId>, NonZero<ActorId>);
pub type AllowancesValue<T> = (NonZero<T>, u32);

/// A sharded map for storing VFT allowances.
///
/// All functions are transactional, meaning if err is returned,
/// state hasn't been changed.
pub struct Allowances<T = Allowance> {
    expiry_period: u32,
    store: ShardedMap<AllowancesKey, AllowancesValue<T>>,
}

impl<T> Allowances<T> {
    /// Default, recommended max shard capacity.
    pub const DEFAULT_MAX_SHARD: usize = 0b11100000000000000000000;

    /// Tries to create a new [`Self`] instance with the given capacities.
    ///
    /// Reuses [`ShardedMap::try_new`] under the hood.
    pub fn try_new(capacities: Vec<usize>, expiry_period: u32) -> Result<Self, AllowancesError> {
        let store = ShardedMap::try_new(capacities)?;

        Ok(Self {
            store,
            expiry_period,
        })
    }

    /// Returns the expiry period for the allowances.
    pub fn expiry_period(&self) -> u32 {
        self.expiry_period
    }

    /// Allocates next shard of underlying sharded map.
    ///
    /// Returns bool indicating if there're unallocated shards left.
    pub fn allocate_next_shard(&mut self) -> bool {
        self.store.alloc_next_shard()
    }

    /// Sets the expiry period for the allowances.
    pub fn set_expiry_period(&mut self, expiry_period: u32) {
        self.expiry_period = expiry_period;
    }

    /// Tries to append a new shard to the underlying sharded map.
    pub fn try_append_shard(&mut self, capacity: usize) -> Result<(), AllowancesError> {
        self.store.try_append_shard(capacity).map_err(Into::into)
    }

    /// Calculates the expiry since a given block number.
    const fn expiry(&self, current_bn: u32) -> u32 {
        self.expiry_period.saturating_add(current_bn)
    }
}

impl<T> Default for Allowances<T> {
    fn default() -> Self {
        unwrap_infallible!(
            Self::try_new(vec![Self::DEFAULT_MAX_SHARD], u32::MAX).map_err(|_| unreachable!())
        )
    }
}

impl<T> Deref for Allowances<T> {
    type Target = ShardedMap<AllowancesKey, AllowancesValue<T>>;

    fn deref(&self) -> &Self::Target {
        &self.store
    }
}

#[cfg(feature = "test")]
impl core::ops::DerefMut for Allowances {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.store
    }
}

impl<T: Clone + Math> Allowances<T> {
    /// Gets the allowance for a given owner and spender.
    ///
    /// Returns ZERO if the allowance is not found.
    pub fn get(&self, owner: NonZero<ActorId>, spender: NonZero<ActorId>) -> T {
        self.deref()
            .get(&(owner, spender))
            .map(|(_, (v, _))| v.clone().into())
            .unwrap_or(Zero::ZERO)
    }

    /// Decreases the allowance for a given owner and spender, updating the expiry.
    ///
    /// If the allowance is at maximum, it will only be set to the new expiry.
    ///
    /// Fails if:
    /// - allowance is insufficient.
    pub fn decrease(
        &mut self,
        owner: NonZero<ActorId>,
        spender: NonZero<ActorId>,
        value: NonZero<T>,
        current_bn: u32,
    ) -> Result<(), AllowancesError> {
        ok_if!(owner == spender);

        let expiry = self.expiry(current_bn);

        let (idx, (allowance, expiration)) = self
            .store
            .get_mut(&(owner, spender))
            .ok_or(AllowancesError::Insufficient(UnderflowError))?;

        if allowance.is_max() {
            *expiration = expiry;
        } else {
            match allowance.clone().try_sub(value) {
                Ok(new_allowance) => {
                    *allowance = new_allowance;
                    *expiration = expiry;
                }
                Err(MathError::Underflow(e)) => Err(e)?,
                Err(MathError::Zero(_)) => {
                    self.store.remove_at(idx, &(owner, spender));
                }
                Err(MathError::Overflow(_)) => unreachable!(),
            };
        }

        Ok(())
    }

    /// Removes the allowance for a given owner and spender and returns the value.
    pub fn remove(
        &mut self,
        owner: NonZero<ActorId>,
        spender: NonZero<ActorId>,
    ) -> Option<AllowancesValue<T>> {
        self.store.remove(&(owner, spender)).map(|(_, v)| v)
    }

    /// Sets the allowance for a given owner and spender, returning the previous value.
    ///
    /// Noop if the owner and spender are the same.
    ///
    /// Max value won't be reducible on decrease. It means infinite allowance.
    ///
    /// Fails if:
    /// - map capacity is exceeded.
    pub fn set(
        &mut self,
        owner: NonZero<ActorId>,
        spender: NonZero<ActorId>,
        value: T,
        current_bn: u32,
    ) -> Result<Option<NonZero<T>>, AllowancesError> {
        ok_if!(owner == spender, None);

        let previous = if let Ok(value) = NonZero::try_new(value) {
            let (_, previous) = self
                .store
                .try_insert((owner, spender), (value, self.expiry(current_bn)))?;

            previous
        } else {
            self.remove(owner, spender)
        };

        Ok(previous.map(|(v, _)| v))
    }
}

#[derive(
    Clone, Debug, Decode, Encode, PartialEq, Eq, PartialOrd, Ord, TypeInfo, thiserror::Error,
)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub enum AllowancesError {
    #[error("insufficient allowance")]
    Insufficient(#[from] UnderflowError),
    #[error("sharded map error: {0}")]
    Map(#[from] ShardedMapError),
}
