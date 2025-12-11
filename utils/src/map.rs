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

//! Awesome ShardedMap module.

use crate::ensure;
use core::{hash::Hash, mem};
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use gstd::collections::HashMap;
use gstd::vec::Vec;

/// A sharded hash map that allows to pick a different shard's capacity, so
/// resulting capacity isn't that much restricted by the HashMap impl.
///
/// Useful for optimal filling of limited storage space.
pub struct ShardedMap<K, V> {
    shards: Vec<(HashMap<K, V>, usize)>,
}

impl<K, V> ShardedMap<K, V> {
    /// Creates new sharded map with given capacities for underlying shards.
    ///
    /// Capacities must be [0b1] (1), [0b11] (3) or [0b111] (7) with any amount of trailing zeroes (14, 28, 56 ...).
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

    /// Returns currently allocated capacity of the map.
    pub fn capacity(&self) -> usize {
        self.shards.iter().map(|(map, _)| map.capacity()).sum()
    }

    /// Returns maximal capacity of the map.
    pub fn max_capacity(&self) -> usize {
        self.shards.iter().map(|(_, c)| *c).sum()
    }

    /// Returns amount of elements in the map.
    pub fn len(&self) -> usize {
        self.shards.iter().map(|(map, _)| map.len()).sum()
    }

    /// Returns bool indicating if the map is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns amount of free space in the map.
    pub fn space(&self) -> usize {
        self.shards
            .iter()
            .map(|(map, _)| map.capacity() - map.len())
            .sum()
    }

    /// Returns bool indicating if the map has free space.
    pub fn has_space(&self) -> bool {
        self.shards
            .iter()
            .any(|(map, _)| map.len() < map.capacity())
    }

    /// Returns result indicating if the map has free space.
    pub fn has_space_err(&self) -> Result<(), ShardedMapError> {
        self.has_space()
            .then_some(())
            .ok_or(ShardedMapError::CapacityOverflow)
    }

    /// Returns iterator over all key-value pairs in the map.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.shards.iter().flat_map(|(map, _)| map.iter())
    }

    /// Returns mutable iterator over all key-value pairs in the map.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut V)> {
        self.shards.iter_mut().flat_map(|(map, _)| map.iter_mut())
    }

    /// Allocates next shard for the map.
    ///
    /// Returns bool indicating if there're unallocated shards left.
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

    /// Clears all shards in the map.
    pub fn clear_shards(&mut self) {
        self.shards.iter_mut().for_each(|(map, _)| map.clear());
    }

    /// Tries to appends a new shard to the map.
    ///
    /// It will require upcoming allocation (see [`Self::alloc_next_shard`]).
    pub fn try_append_shard(&mut self, capacity: usize) -> Result<(), ShardedMapError> {
        ensure!(
            Self::is_valid_capacity(capacity),
            ShardedMapError::InvalidCapacity
        );

        self.shards.push((HashMap::new(), capacity));

        Ok(())
    }

    /// Helper function to `find_map` shards.
    fn find_map<'a, T: 'a, F>(&'a self, f: F) -> Option<(ShardIdx, T)>
    where
        F: Fn((usize, &'a HashMap<K, V>)) -> Option<T>,
    {
        self.shards
            .iter()
            .enumerate()
            .find_map(|(idx, (map, _))| f((idx, map)).map(|v| (ShardIdx(idx), v)))
    }

    /// Helper function to `find_map` shards mutably.
    fn find_map_mut<'a, T: 'a, F>(&'a mut self, mut f: F) -> Option<(ShardIdx, T)>
    where
        F: FnMut((usize, &'a mut HashMap<K, V>)) -> Option<T>,
    {
        self.shards
            .iter_mut()
            .enumerate()
            .find_map(|(idx, (map, _))| f((idx, map)).map(|v| (ShardIdx(idx), v)))
    }

    /// Helper function to check if the given capacity is valid.
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
    /// Returns a reference to the value under the given key with its shard index.
    pub fn get(&self, key: &K) -> Option<(ShardIdx, &V)> {
        self.find_map(|(_, map)| map.get(key))
    }
    /// Returns a reference to the value under the given key at specific shard.
    pub fn get_at(&self, idx: ShardIdx, key: &K) -> Option<&V> {
        self.shards[idx.0].0.get(key)
    }

    /// Returns a mut reference to the value under the given key with its shard index.
    pub fn get_mut(&mut self, key: &K) -> Option<(ShardIdx, &mut V)> {
        self.find_map_mut(|(_, map)| map.get_mut(key))
    }

    /// Returns a mut reference to the value under the given key at specific shard.
    pub fn get_mut_at(&mut self, idx: ShardIdx, key: &K) -> Option<&mut V> {
        self.shards[idx.0].0.get_mut(key)
    }

    /// Removes the value under the given key, returning it with its shard index.
    pub fn remove(&mut self, key: &K) -> Option<(ShardIdx, V)> {
        self.find_map_mut(|(_, map)| map.remove(key))
    }

    /// Removes the value under the given key at given shard index, returning it.
    pub fn remove_at(&mut self, idx: ShardIdx, key: &K) -> Option<V> {
        self.shards[idx.0].0.remove(key)
    }

    /// Tries to insert a new key-value pair into the map.
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

    /// Tries to insert a new key-value pair with guarantee that
    /// the key does not exist in any shard yet.
    ///
    /// # Safety
    /// The caller must ensure that the key does not exist in any shard.
    pub unsafe fn try_insert_new(&mut self, key: K, value: V) -> Result<ShardIdx, ShardedMapError> {
        self.find_map_mut(|(_, map)| (map.len() < map.capacity()).then_some(map))
            .map(|(idx, map)| {
                map.insert(key, value);
                idx
            })
            .ok_or(ShardedMapError::CapacityOverflow)
    }

    /// Tries to insert a new key-value pair at specific shard with guarantee
    /// that the key does not exist in any shard yet.
    ///
    /// # Safety
    /// The caller must ensure that the key does not exist in any shard.
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

/// Shard index type.
///
/// Used to identify a shard in the sharded map.
#[derive(Debug)]
pub struct ShardIdx(usize);

impl ShardIdx {
    /// Clones the shard index.
    ///
    /// # Safety
    /// The caller must ensure that the index is still available for the operation after use.
    pub unsafe fn cloned(&self) -> Self {
        Self(self.0)
    }
}

/// Error type for ShardedMap operations.
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Decode, Encode, TypeInfo, thiserror::Error,
)]
#[codec(crate = parity_scale_codec)]
#[scale_info(crate = scale_info)]
pub enum ShardedMapError {
    #[error("capacity overflow")]
    CapacityOverflow,
    #[error("invalid capacity")]
    InvalidCapacity,
}
