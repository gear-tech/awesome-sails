// This file is part of Gear.

// Copyright (C) 2026 Gear Technologies Inc.
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

use crate::MessageStorage;
use core::convert::Infallible;
use error::TrackerError;
pub use sails_rs::collections::BTreeMap;
use sails_rs::prelude::*;

impl<T> MessageStorage<T> for BTreeMap<MessageId, T> {
    type Error = Infallible;

    fn insert(&mut self, msg_id: MessageId, status: T) -> Result<(), Self::Error> {
        self.insert(msg_id, status);
        Ok(())
    }

    fn get(&self, msg_id: &MessageId) -> Option<&T> {
        self.get(msg_id)
    }

    fn get_mut(&mut self, msg_id: &MessageId) -> Option<&mut T> {
        self.get_mut(msg_id)
    }

    fn remove(&mut self, msg_id: &MessageId) -> Option<T> {
        self.remove(msg_id)
    }

    fn get_statuses(&self, query: Option<crate::Pagination>) -> Vec<(MessageId, T)>
    where
        T: Clone,
    {
        let (offset, limit) = crate::Pagination::range(query);
        self.iter()
            .skip(offset)
            .take(limit)
            .map(|(&id, s)| (id, s.clone()))
            .collect()
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn clear(&mut self) {
        self.clear();
    }
}

/// Storage implementation using fixed-size arrays.
#[derive(Debug, Encode, Decode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
pub struct FixedStorage<T, const N: usize> {
    /// Array of tracked message identifiers.
    pub ids: [MessageId; N],
    /// Array of optional message statuses corresponding to the IDs.
    pub statuses: [Option<T>; N],
    /// Current number of tracked messages.
    pub len: u32,
}

impl<T, const N: usize> Default for FixedStorage<T, N> {
    fn default() -> Self {
        Self {
            ids: [MessageId::zero(); N],
            statuses: [const { None }; N],
            len: 0,
        }
    }
}

impl<T, const N: usize> MessageStorage<T> for FixedStorage<T, N> {
    type Error = TrackerError;

    fn insert(&mut self, msg_id: MessageId, status: T) -> Result<(), Self::Error> {
        let length = self.len as usize;
        match self.ids[..length].binary_search(&msg_id) {
            Ok(idx) => {
                self.statuses[idx] = Some(status);
                Ok(())
            }
            Err(idx) => {
                if length >= N {
                    return Err(TrackerError::CapacityExceeded);
                }
                self.ids.copy_within(idx..length, idx + 1);

                for i in (idx..length).rev() {
                    self.statuses[i + 1] = self.statuses[i].take();
                }

                self.ids[idx] = msg_id;
                self.statuses[idx] = Some(status);
                self.len += 1;
                Ok(())
            }
        }
    }

    fn get(&self, msg_id: &MessageId) -> Option<&T> {
        self.ids[..self.len as usize]
            .binary_search(msg_id)
            .ok()
            .and_then(|idx| self.statuses[idx].as_ref())
    }

    fn get_mut(&mut self, msg_id: &MessageId) -> Option<&mut T> {
        self.ids[..self.len as usize]
            .binary_search(msg_id)
            .ok()
            .and_then(|idx| self.statuses[idx].as_mut())
    }

    fn remove(&mut self, msg_id: &MessageId) -> Option<T> {
        let length = self.len as usize;
        if let Ok(idx) = self.ids[..length].binary_search(msg_id) {
            let status = self.statuses[idx].take();

            self.ids.copy_within(idx + 1..length, idx);

            for i in idx..(length - 1) {
                self.statuses[i] = self.statuses[i + 1].take();
            }

            self.len -= 1;
            status
        } else {
            None
        }
    }

    fn get_statuses(&self, query: Option<crate::Pagination>) -> Vec<(MessageId, T)>
    where
        T: Clone,
    {
        let length = self.len as usize;
        let (offset, limit) = crate::Pagination::range(query);
        self.ids[..length]
            .iter()
            .zip(self.statuses[..length].iter().flatten())
            .skip(offset)
            .take(limit)
            .map(|(&id, s)| (id, s.clone()))
            .collect()
    }

    fn len(&self) -> usize {
        self.len as usize
    }

    fn clear(&mut self) {
        for i in 0..self.len as usize {
            self.statuses[i] = None;
        }
        self.len = 0;
    }
}

/// Errors that can occur during storage operations.
pub mod error {
    use sails_rs::prelude::*;

    #[derive(Debug, Decode, Encode, TypeInfo, ReflectHash, thiserror::Error)]
    #[codec(crate = sails_rs::scale_codec)]
    #[reflect_hash(crate = sails_rs)]
    pub enum TrackerError {
        /// Indicates that the fixed storage capacity has been exceeded.
        #[error("Capacity exceeded")]
        CapacityExceeded,
    }
}
