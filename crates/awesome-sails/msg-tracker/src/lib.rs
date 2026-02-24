#![no_std]

use awesome_sails_storage::InfallibleStorageMut;
use core::marker::PhantomData;
use sails_rs::prelude::*;

pub mod storage;

/// Tracker for asynchronous messages and their statuses.
pub struct MsgTracker<T, S>
where
    S: InfallibleStorageMut,
    S::Item: MessageStorage<T>,
{
    storage: S,
    _phantom: PhantomData<T>,
}

impl<T, S> MsgTracker<T, S>
where
    S: InfallibleStorageMut,
    S::Item: MessageStorage<T>,
{
    /// Creates a new `MsgTracker` instance.
    pub fn new(storage: S) -> Self {
        Self {
            storage,
            _phantom: PhantomData,
        }
    }

    /// Starts tracking a message with the given ID and status.
    pub fn insert(&mut self, msg_id: MessageId, status: T) -> Result<(), TrackerError> {
        self.storage.get_mut().insert(msg_id, status)
    }

    /// Retrieves the status of a message.
    pub fn get_status(&self, msg_id: &MessageId) -> Option<T>
    where
        T: Clone,
    {
        self.storage.get().get(msg_id).cloned()
    }

    /// Updates the status of a tracked message.
    ///
    /// Returns `Ok(true)` if the message was already tracked and its status was updated.
    pub fn update_status(&mut self, msg_id: MessageId, status: T) -> Result<bool, TrackerError> {
        let mut storage = self.storage.get_mut();
        if storage.get(&msg_id).is_some() {
            storage.insert(msg_id, status)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Stops tracking a message and returns its final status.
    pub fn remove(&mut self, msg_id: &MessageId) -> Option<T> {
        self.storage.get_mut().remove(msg_id)
    }

    /// Returns the number of messages currently being tracked.
    pub fn len(&self) -> usize {
        self.storage.get().len()
    }

    /// Checks if no messages are being tracked.
    pub fn is_empty(&self) -> bool {
        self.storage.get().is_empty()
    }

    /// Clears all message statuses from the tracker.
    pub fn clear(&mut self) {
        self.storage.get_mut().clear();
    }
}

/// Trait for message status storage.
pub trait MessageStorage<T> {
    /// Inserts a message ID and its status.
    fn insert(&mut self, msg_id: MessageId, status: T) -> Result<(), TrackerError>;

    /// Returns a reference to the status associated with the message ID.
    fn get(&self, msg_id: &MessageId) -> Option<&T>;

    /// Returns a mutable reference to the status associated with the message ID.
    fn get_mut(&mut self, msg_id: &MessageId) -> Option<&mut T>;

    /// Removes the message ID and its status.
    fn remove(&mut self, msg_id: &MessageId) -> Option<T>;

    /// Returns the number of tracked messages.
    fn len(&self) -> usize;

    /// Returns `true` if no messages are tracked.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clears all tracked messages.
    fn clear(&mut self);
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
