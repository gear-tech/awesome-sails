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

//! Module for balances-related VFT logic.

use crate::Balance;
use awesome_sails_utils::{
    ensure,
    map::{ShardedMap, ShardedMapError},
    math::{CheckedMath, Math, MathError, NonZero, OverflowError, UnderflowError, Zero, ZeroError},
    ok_if, unwrap_infallible,
};
use core::{mem, ops::Deref};
use sails_rs::{ActorId, Decode, Encode, TypeInfo, U256, vec, vec::Vec};

pub type BalancesKey = NonZero<ActorId>;
pub type BalancesValue<T> = NonZero<T>;

/// A sharded map for storing VFT balances.
///
/// All functions are transactional, meaning if err is returned,
/// state hasn't been changed.
pub struct Balances<T = Balance> {
    minimum_balance: T,
    store: ShardedMap<BalancesKey, BalancesValue<T>>,
    total: U256,
    unused: U256,
}

impl<T> Balances<T> {
    /// Default, recommended max shard capacity.
    pub const DEFAULT_MAX_SHARD: usize = 0b111000000000000000000000;

    /// Tries to create a new [`Self`] instance with the given capacities.
    ///
    /// Reuses [`ShardedMap::try_new`] under the hood.
    pub fn try_new(capacities: Vec<usize>, minimum_balance: T) -> Result<Self, BalancesError> {
        let store = ShardedMap::try_new(capacities)?;

        Ok(Self {
            store,
            minimum_balance,
            total: U256::zero(),
            unused: U256::zero(),
        })
    }

    /// Returns the minimum balance.
    pub fn minimum_balance(&self) -> &T {
        &self.minimum_balance
    }

    /// Returns the total supply of the balances.
    pub fn total_supply(&self) -> U256 {
        self.total
    }

    /// Returns the unused value of the balances: nobody's balance,
    /// created during burns of dust (balances below minimum).
    pub fn unused_value(&self) -> U256 {
        self.unused
    }

    /// Allocates next shard of underlying sharded map.
    ///
    /// Returns bool indicating if there're unallocated shards left.
    pub fn allocate_next_shard(&mut self) -> bool {
        self.store.alloc_next_shard()
    }

    /// Sets the new minimum balance.
    pub fn set_minimum_balance(&mut self, minimum_balance: T) {
        self.minimum_balance = minimum_balance;
    }

    /// Tries to append a new shard to the underlying sharded map.
    pub fn try_append_shard(&mut self, capacity: usize) -> Result<(), BalancesError> {
        self.store.try_append_shard(capacity).map_err(Into::into)
    }
}

impl<T: Zero> Default for Balances<T> {
    fn default() -> Self {
        unwrap_infallible!(
            Self::try_new(vec![Self::DEFAULT_MAX_SHARD; 2], T::ZERO).map_err(|_| unreachable!())
        )
    }
}

impl<T> Deref for Balances<T> {
    type Target = ShardedMap<NonZero<ActorId>, NonZero<T>>;

    fn deref(&self) -> &Self::Target {
        &self.store
    }
}

#[cfg(feature = "test")]
impl core::ops::DerefMut for Balances {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.store
    }
}

#[cfg(feature = "test")]
impl<T> Balances<T> {
    /// Sets the total supply.
    pub fn set_total_supply(&mut self, total: U256) {
        self.total = total;
    }

    /// Sets the unused value.
    pub fn set_unused_value(&mut self, unused: U256) {
        self.unused = unused;
    }
}

impl<T: Clone + Math + PartialOrd> Balances<T>
where
    U256: From<T>,
{
    /// Gets the balance for a given account.
    ///
    /// Returns ZERO if the balance is not found.
    pub fn get(&self, account: NonZero<ActorId>) -> T {
        self.store
            .get(&account)
            .map(|(_, v)| v.clone().into())
            .unwrap_or(Zero::ZERO)
    }

    /// Burns a specified amount of value from the balance of a given account,
    /// reducing the total supply.
    ///
    /// If the balance after burning is below the minimum balance, the account
    /// is removed from the store and unused value increased accordingly.
    ///
    /// Fails if:
    /// - balance is insufficient;
    /// - unused balance overflows.
    pub fn burn(
        &mut self,
        account: NonZero<ActorId>,
        value: NonZero<T>,
    ) -> Result<(), BalancesError> {
        let (idx, balance) = self.store.get_mut(&account).ok_or(UnderflowError)?;

        match balance.clone().try_sub(value.clone()) {
            Ok(remaining) if remaining >= self.minimum_balance => {
                *balance = remaining;
            }
            Ok(remaining) /* if remaining < self.minimum_balance */ => {
                self.unused = self.unused.checked_add_err(remaining.cast())?;
                self.store.remove_at(idx, &account);
            },
            Err(MathError::Zero(_)) => {
                self.store.remove_at(idx, &account);
            }
            Err(MathError::Overflow(e)) => Err(e)?,
            Err(MathError::Underflow(e)) => Err(e)?,
        };

        self.total = unwrap_infallible!(
            self.total
                .checked_sub(value.cast())
                .ok_or_else(|| unreachable!())
        );

        Ok(())
    }

    /// Burns all value from the balance of a given account,
    /// reducing the total supply.
    ///
    /// Returns the amount of the burned value.
    pub fn burn_all(&mut self, account: NonZero<ActorId>) -> T {
        let Some(value) = self.store.remove(&account).map(|(_, v)| v.into_inner()) else {
            return Zero::ZERO;
        };

        self.total = unwrap_infallible!(
            self.total
                .checked_sub(value.clone().into())
                .ok_or_else(|| unreachable!())
        );

        value
    }

    /// Burns all unused value, reducing the total supply and returning the burned amount.
    pub fn burn_unused(&mut self) -> U256 {
        self.total = unwrap_infallible!(
            self.total
                .checked_sub(self.unused)
                .ok_or_else(|| unreachable!())
        );

        mem::take(&mut self.unused)
    }

    /// Mints a specified amount of value for a given account, increasing the total supply.
    ///
    /// Fails if:
    /// - new account balance is below the minimum;
    /// - new account balance overflows;
    /// - total supply overflows;
    /// - map capacity exceed.
    pub fn mint(
        &mut self,
        account: NonZero<ActorId>,
        value: NonZero<T>,
    ) -> Result<(), BalancesError> {
        let new_total = self.total.checked_add_err(value.clone().cast())?;

        match self.store.get_mut(&account) {
            Some((_, balance)) => {
                let new_balance = balance.clone().try_add(value)?;

                ensure!(
                    new_balance >= self.minimum_balance,
                    BalancesError::BelowMinimum
                );

                *balance = new_balance;
            }
            _ if value < self.minimum_balance => Err(BalancesError::BelowMinimum)?,
            _ => unsafe {
                self.store.try_insert_new(account, value)?;
            },
        }

        self.total = new_total;

        Ok(())
    }

    /// Transfers a specified amount of value from one account to another.
    ///
    /// If `to` is zero, it's equivalent to [`Self::burn`].
    ///
    /// Fails if:
    /// - `from` balance is insufficient;
    /// - new `to` balance is below the minimum;
    /// - new `to` balance overflows;
    /// - total supply overflows;
    /// - unused balance overflows;
    /// - map capacity exceed.
    pub fn transfer(
        &mut self,
        from: NonZero<ActorId>,
        to: ActorId,
        value: NonZero<T>,
    ) -> Result<(), BalancesError> {
        let Ok(to) = NonZero::try_from(to) else {
            return self.burn(from, value);
        };

        ok_if!(from == to);

        let (idx_from, balance_from) = self.store.get(&from).ok_or(UnderflowError)?;

        let mut new_balance_from = None;
        let mut new_unused = None;

        match balance_from.clone().try_sub(value.clone()) {
            Ok(remaining_from) if remaining_from >= self.minimum_balance => {
                new_balance_from = Some(remaining_from);
            }
            Ok(remaining_from) /* if remaining_from < self.minimum_balance */ => {
                new_unused = Some(self.unused.checked_add_err(remaining_from.cast())?);
                /* `from` to be removed */
            }
            Err(MathError::Zero(_)) => { /* no `new_unused`, `from` to be removed */}
            Err(MathError::Overflow(e)) => Err(e)?,
            Err(MathError::Underflow(e)) => Err(e)?,
        };

        let mut insert_balance_to = None;

        match self.store.get_mut(&to) {
            Some((_, balance_to)) => {
                let new_balance_to = balance_to.clone().try_add(value)?;

                ensure!(
                    new_balance_to >= self.minimum_balance,
                    BalancesError::BelowMinimum
                );

                *balance_to = new_balance_to;
            }
            None => {
                if new_balance_from.is_some() {
                    self.store.has_space_err()?;
                }

                ensure!(value >= self.minimum_balance, BalancesError::BelowMinimum);

                insert_balance_to = Some(value);
            }
        };

        if let Some(new_balance_from) = new_balance_from {
            let balance_from = unwrap_infallible!(
                self.store
                    .get_mut_at(idx_from, &from)
                    .ok_or_else(|| unreachable!())
            );

            *balance_from = new_balance_from;

            if let Some(balance_to) = insert_balance_to {
                unsafe {
                    unwrap_infallible!(
                        self.store
                            .try_insert_new(to, balance_to)
                            .map_err(|_| unreachable!())
                    );
                }
            }
        } else {
            self.store.remove_at(unsafe { idx_from.cloned() }, &from);

            if let Some(balance_to) = insert_balance_to {
                unsafe {
                    unwrap_infallible!(
                        self.store
                            .try_insert_new_at(idx_from, to, balance_to)
                            .map_err(|_| unreachable!())
                    );
                }
            }
        };

        if let Some(unused) = new_unused {
            self.unused = unused;
        }

        Ok(())
    }

    /// Transfers all value from one account to another, returning the amount
    /// of the transferred value.
    ///
    /// If `to` is zero, it's equivalent to [`Self::burn_all`].
    ///
    /// Fails if:
    /// - new `to` balance is below the minimum;
    /// - new `to` balance overflows.
    pub fn transfer_all(
        &mut self,
        from: NonZero<ActorId>,
        to: NonZero<ActorId>,
    ) -> Result<T, BalancesError> {
        let Some((idx_from, balance_from)) = self.store.get(&from).map(|(i, b)| (i, b.clone()))
        else {
            return Ok(Zero::ZERO);
        };

        ok_if!(from == to, balance_from);

        let mut insert_balance_to = None;

        if let Some((_, balance_to)) = self.store.get_mut(&to) {
            let new_balance_to = balance_to.clone().try_add(balance_from)?;

            ensure!(
                new_balance_to >= self.minimum_balance,
                BalancesError::BelowMinimum
            );

            *balance_to = new_balance_to;
        } else {
            ensure!(
                balance_from >= self.minimum_balance,
                BalancesError::BelowMinimum
            );

            insert_balance_to = Some(balance_from);
        }

        let balance_from = unwrap_infallible!(
            self.store
                .remove_at(unsafe { idx_from.cloned() }, &from)
                .ok_or_else(|| unreachable!())
        );

        if let Some(balance_to) = insert_balance_to {
            unsafe {
                unwrap_infallible!(
                    self.store
                        .try_insert_new_at(idx_from, to, balance_to)
                        .map_err(|_| unreachable!())
                );
            }
        }

        Ok(balance_from.into())
    }
}

#[derive(Clone, Debug, Decode, Encode, TypeInfo, thiserror::Error)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub enum BalancesError {
    #[error("balance below minimum")]
    BelowMinimum,
    #[error("insufficient balance")]
    Insufficient(#[from] UnderflowError),
    #[error("sharded map error: {0}")]
    Map(#[from] ShardedMapError),
    #[error("balance or supply overflow")]
    Overflow(#[from] OverflowError),
    /// Addition ended up in zero.
    ///
    /// Should never happen with proper (unsigned) balance type used.
    #[error("unexpected zero value")]
    Zero(#[from] ZeroError),
}

impl From<MathError> for BalancesError {
    fn from(err: MathError) -> Self {
        match err {
            MathError::Underflow(e) => e.into(),
            MathError::Overflow(e) => e.into(),
            MathError::Zero(e) => e.into(),
        }
    }
}
