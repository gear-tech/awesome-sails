use crate::MessageStorage;
pub use sails_rs::collections::BTreeMap;
use sails_rs::prelude::*;

impl<T> MessageStorage<T> for BTreeMap<MessageId, T> {
    fn insert(&mut self, msg_id: MessageId, status: T) -> Result<(), TrackerError> {
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
///
/// Uses binary search on a sorted array of message IDs.
#[derive(Debug, Encode, Decode, TypeInfo)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub struct FixedStorage<T, const N: usize> {
    pub ids: [MessageId; N],
    pub statuses: [Option<T>; N],
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
    fn insert(&mut self, msg_id: MessageId, status: T) -> Result<(), TrackerError> {
        match self.ids[..self.len as usize].binary_search(&msg_id) {
            Ok(idx) => {
                self.statuses[idx] = Some(status);
                Ok(())
            }
            Err(idx) => {
                if (self.len as usize) >= N {
                    return Err(TrackerError::CapacityExceeded);
                }
                // Shift elements to the right to maintain sorted order
                for i in (idx..self.len as usize).rev() {
                    self.ids[i + 1] = self.ids[i];
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
        if let Ok(idx) = self.ids[..self.len as usize].binary_search(msg_id) {
            let status = self.statuses[idx].take();
            // Shift elements to the left
            for i in idx..(self.len as usize - 1) {
                self.ids[i] = self.ids[i + 1];
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
        let (offset, limit) = crate::Pagination::range(query);
        self.ids[..self.len as usize]
            .iter()
            .zip(self.statuses[..self.len as usize].iter().flatten())
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

/// Errors that can occur during message tracking.
#[derive(Debug, Decode, Encode, TypeInfo, thiserror::Error)]
#[codec(crate = sails_rs::scale_codec)]
#[scale_info(crate = sails_rs::scale_info)]
pub enum TrackerError {
    /// Indicates that the fixed storage capacity has been exceeded.
    #[error("Capacity exceeded")]
    CapacityExceeded,
}
