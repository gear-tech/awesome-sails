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

//! Defines the `ShardedMap` data structure.
//!
//! `ShardedMap` is a specialized hash map implementation that distributes data across multiple
//! internal "shards" (HashMaps) with predefined capacities. This allows for fine-grained control
//! over storage allocation, enabling optimal filling of limited storage space, such as in
//! smart contract environments.

use crate::ensure;
use alloc::vec::Vec;
use core::{hash::Hash, mem};
use hashbrown::HashMap;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;

/// A sharded hash map implementation.
///
/// This structure maintains a vector of separate `HashMap` instances (shards), each with its own
/// capacity. It allows for efficient storage usage by enabling different capacity limits for
/// different shards.
pub struct ShardedMap<K, V> {
    shards: Vec<(HashMap<K, V>, usize)>,
}

impl<K, V> ShardedMap<K, V> {
    /// Creates a new `ShardedMap` with the specified capacities for its shards.
    ///
    /// # Arguments
    ///
    /// * `capacities` - A vector of capacities for each shard.
    ///
    /// # Returns
    ///
    /// A `Result` containing the new `ShardedMap` if successful, or a `ShardedMapError`.
    ///
    /// # Errors
    ///
    /// Returns `ShardedMapError::InvalidCapacity` if any of the provided capacities are invalid.
    /// Valid capacities must be of the form `2^n - 1` (e.g., 1, 3, 7) potentially shifted left.
    pub fn try_new(mut capacities: Vec<usize>) -> Result<Self, ShardedMapError> {
        ensure!(
            capacities.iter().all(|&c| Self::is_valid_capacity(c)),
            ShardedMapError::InvalidCapacity
        );

        capacities.sort();

        let shards = capacities
            .into_iter()
            .rev()
            .map(|c| (HashMap::new(), c))
            .collect();

        Ok(Self { shards })
    }

    /// Returns the current total allocated capacity of the map.
    ///
    /// This is the sum of the current capacities of all internal shards.
    pub fn capacity(&self) -> usize {
        self.shards.iter().map(|(map, _)| map.capacity()).sum()
    }

    /// Returns the maximum potential capacity of the map.
    ///
    /// This is the sum of the configured maximum capacities for all shards.
    pub fn max_capacity(&self) -> usize {
        self.shards.iter().map(|(_, c)| *c).sum()
    }

    /// Returns the total number of elements currently in the map.
    pub fn len(&self) -> usize {
        self.shards.iter().map(|(map, _)| map.len()).sum()
    }

    /// Returns `true` if the map contains no elements.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the total remaining space available in the map.
    ///
    /// This is calculated as the sum of (capacity - length) for each shard.
    pub fn space(&self) -> usize {
        self.shards
            .iter()
            .map(|(map, _)| map.capacity() - map.len())
            .sum()
    }

    /// Returns `true` if there is space available for at least one more element.
    pub fn has_space(&self) -> bool {
        self.shards
            .iter()
            .any(|(map, _)| map.len() < map.capacity())
    }

    /// Checks if there is space available and returns a `Result`.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if there is space.
    /// * `Err(ShardedMapError::CapacityOverflow)` if the map is full.
    pub fn has_space_err(&self) -> Result<(), ShardedMapError> {
        self.has_space()
            .then_some(())
            .ok_or(ShardedMapError::CapacityOverflow)
    }

    /// Returns an iterator over all key-value pairs in the map.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.shards.iter().flat_map(|(map, _)| map.iter())
    }

    /// Returns a mutable iterator over all key-value pairs in the map.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut V)> {
        self.shards.iter_mut().flat_map(|(map, _)| map.iter_mut())
    }

    /// Allocates the next available shard if needed.
    ///
    /// If a shard has 0 capacity allocated but a configured limit, this allocates memory for it.
    ///
    /// # Returns
    ///
    /// `true` if there are still unallocated shards remaining after this operation.
    pub fn alloc_next_shard(&mut self) -> bool {
        let idx = self
            .shards
            .iter_mut()
            .enumerate()
            .find_map(|(i, (map, cap))| {
                (map.capacity() == 0).then(|| {
                    *map = HashMap::with_capacity(*cap);
                    i
                })
            });

        idx.is_some_and(|i| i != self.shards.len() - 1)
    }

    /// Clears all elements from all shards in the map.
    pub fn clear_shards(&mut self) {
        self.shards.iter_mut().for_each(|(map, _)| map.clear());
    }

    /// Attempts to append a new shard with the given capacity to the map.
    ///
    /// # Arguments
    ///
    /// * `capacity` - The capacity for the new shard.
    ///
    /// # Errors
    ///
    /// Returns `ShardedMapError::InvalidCapacity` if the provided capacity is invalid.
    pub fn try_append_shard(&mut self, capacity: usize) -> Result<(), ShardedMapError> {
        ensure!(
            Self::is_valid_capacity(capacity),
            ShardedMapError::InvalidCapacity
        );

        self.shards.push((HashMap::new(), capacity));

        Ok(())
    }

    /// Helper function to find a shard and map a value from it.
    fn find_map<'a, T: 'a, F>(&'a self, f: F) -> Option<(ShardIdx, T)>
    where
        F: Fn((usize, &'a HashMap<K, V>)) -> Option<T>,
    {
        self.shards
            .iter()
            .enumerate()
            .find_map(|(idx, (map, _))| f((idx, map)).map(|v| (ShardIdx(idx), v)))
    }

    /// Helper function to find a shard and map a mutable value from it.
    fn find_map_mut<'a, T: 'a, F>(&'a mut self, mut f: F) -> Option<(ShardIdx, T)>
    where
        F: FnMut((usize, &'a mut HashMap<K, V>)) -> Option<T>,
    {
        self.shards
            .iter_mut()
            .enumerate()
            .find_map(|(idx, (map, _))| f((idx, map)).map(|v| (ShardIdx(idx), v)))
    }

    /// Helper function to validate capacity values.
    const fn is_valid_capacity(mut n: usize) -> bool {
        if n == 0 || n == usize::MAX {
            return false;
        }

        let shift_for = n.trailing_zeros();

        n >>= shift_for;

        if (n & (n + 1)) != 0 {
            return false;
        }

        shift_for == 0 || n.trailing_ones() >= 3
    }
}

impl<K: Eq + Hash, V> ShardedMap<K, V> {
    /// Returns a reference to the value corresponding to the key, along with its shard index.
    pub fn get(&self, key: &K) -> Option<(ShardIdx, &V)> {
        self.find_map(|(_, map)| map.get(key))
    }
    /// Returns a reference to the value corresponding to the key in a specific shard.
    pub fn get_at(&self, idx: ShardIdx, key: &K) -> Option<&V> {
        self.shards[idx.0].0.get(key)
    }

    /// Returns a mutable reference to the value corresponding to the key, along with its shard index.
    pub fn get_mut(&mut self, key: &K) -> Option<(ShardIdx, &mut V)> {
        self.find_map_mut(|(_, map)| map.get_mut(key))
    }

    /// Returns a mutable reference to the value corresponding to the key in a specific shard.
    pub fn get_mut_at(&mut self, idx: ShardIdx, key: &K) -> Option<&mut V> {
        self.shards[idx.0].0.get_mut(key)
    }

    /// Removes a key from the map, returning the value and its shard index if the key was previously in the map.
    pub fn remove(&mut self, key: &K) -> Option<(ShardIdx, V)> {
        self.find_map_mut(|(_, map)| map.remove(key))
    }

    /// Removes a key from a specific shard, returning the value if the key was previously in that shard.
    pub fn remove_at(&mut self, idx: ShardIdx, key: &K) -> Option<V> {
        self.shards[idx.0].0.remove(key)
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the map did not have this key present, [`None`] is returned.
    /// If the map did have this key present, the value is updated, and the old value is returned.
    ///
    /// # Errors
    ///
    /// Returns `ShardedMapError::CapacityOverflow` if the key is new and there is no space in any shard.
    pub fn try_insert(
        &mut self,
        key: K,
        value: V,
    ) -> Result<(ShardIdx, Option<V>), ShardedMapError> {
        let mut available_map = None;

        if let Some((idx, prev_value_mut)) = self.find_map_mut(|(idx, map)| {
            if available_map.is_none() && map.len() < map.capacity() {
                available_map = Some(idx);
            };

            map.get_mut(&key)
        }) {
            return Ok((idx, Some(mem::replace(prev_value_mut, value))));
        };

        available_map
            .map(|idx| (ShardIdx(idx), self.shards[idx].0.insert(key, value)))
            .ok_or(ShardedMapError::CapacityOverflow)
    }

    /// Inserts a new key-value pair into the map, assuming the key does not already exist.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the key does not exist in any shard.
    ///
    /// # Errors
    ///
    /// Returns `ShardedMapError::CapacityOverflow` if there is no space available in any shard.
    pub unsafe fn try_insert_new(&mut self, key: K, value: V) -> Result<ShardIdx, ShardedMapError> {
        self.find_map_mut(|(_, map)| (map.len() < map.capacity()).then_some(map))
            .map(|(idx, map)| {
                map.insert(key, value);
                idx
            })
            .ok_or(ShardedMapError::CapacityOverflow)
    }

    /// Inserts a new key-value pair into a specific shard, assuming the key does not already exist.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the key does not exist in any shard.
    ///
    /// # Errors
    ///
    /// Returns `ShardedMapError::CapacityOverflow` if the specified shard is full.
    pub unsafe fn try_insert_new_at(
        &mut self,
        idx: ShardIdx,
        key: K,
        value: V,
    ) -> Result<(), ShardedMapError> {
        let map = &mut self.shards[idx.0].0;

        ensure!(
            map.len() < map.capacity(),
            ShardedMapError::CapacityOverflow
        );

        map.insert(key, value);

        Ok(())
    }
}

/// Represents the index of a shard within the `ShardedMap`.
///
/// This is used to reference the specific internal HashMap where an item is stored.
#[derive(Debug)]
pub struct ShardIdx(usize);

impl ShardIdx {
    /// Creates a copy of the shard index.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the index remains valid and applicable to the current state of the map.
    pub unsafe fn cloned(&self) -> Self {
        Self(self.0)
    }
}

/// Errors that can occur during `ShardedMap` operations.
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Decode, Encode, TypeInfo, thiserror::Error,
)]
#[codec(crate = parity_scale_codec)]
#[scale_info(crate = scale_info)]
pub enum ShardedMapError {
    /// Indicates that the operation failed because the map (or shard) is at full capacity.
    #[error("capacity overflow")]
    CapacityOverflow,
    /// Indicates that an invalid capacity value was provided.
    #[error("invalid capacity")]
    InvalidCapacity,
}
