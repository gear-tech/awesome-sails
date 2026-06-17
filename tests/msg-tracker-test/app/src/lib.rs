#![no_std]

use awesome_sails::msg_tracker::{
    MsgTracker, Pagination, TrackerError,
    storage::{BTreeMap, FixedStorage},
};
use sails_rs::{cell::RefCell, prelude::*};

#[derive(Clone, Encode, Decode, TypeInfo, PartialEq, Debug, ReflectHash)]
#[codec(crate = sails_rs::scale_codec)]
#[reflect_hash(crate = sails_rs)]
pub enum OpStatus {
    Pending,
    Completed,
}

#[derive(Debug, Decode, Encode, TypeInfo, PartialEq, ReflectHash)]
#[codec(crate = sails_rs::scale_codec)]
#[reflect_hash(crate = sails_rs)]
pub enum CounterError {
    OperationNotFound,
    AlreadyCompleted,
}

pub struct DynamicCounter<'a> {
    tracker: MsgTracker<OpStatus, &'a RefCell<BTreeMap<MessageId, OpStatus>>>,
    counter: &'a RefCell<u32>,
}

#[sails_rs::service]
impl DynamicCounter<'_> {
    #[export]
    pub fn request_increment(&mut self) -> MessageId {
        let id = Syscall::message_id();
        self.tracker.insert(id, OpStatus::Pending).unwrap();
        id
    }

    #[export(unwrap_result)]
    pub fn confirm_increment(&mut self, id: MessageId) -> Result<(), CounterError> {
        match self.tracker.get_status(&id) {
            Some(OpStatus::Pending) => {
                *self.counter.borrow_mut() += 1;
                self.tracker.update_status(id, OpStatus::Completed).unwrap();
                Ok(())
            }
            Some(OpStatus::Completed) => Err(CounterError::AlreadyCompleted),
            None => Err(CounterError::OperationNotFound),
        }
    }

    #[export]
    pub fn get_val(&self) -> u32 {
        *self.counter.borrow()
    }

    #[export]
    pub fn get_status(&self, id: MessageId) -> Option<OpStatus> {
        self.tracker.get_status(&id)
    }

    #[export]
    pub fn get_statuses(&self, query: Option<Pagination>) -> Vec<(MessageId, OpStatus)> {
        self.tracker.get_statuses(query)
    }

    #[export]
    pub fn update_dynamic(&mut self, id: MessageId, status: OpStatus) -> bool {
        self.tracker.update_status(id, status).unwrap()
    }

    #[export]
    pub fn clear_dynamic(&mut self) {
        self.tracker.clear();
    }
}

pub struct FixedCounter<'a> {
    tracker: MsgTracker<OpStatus, &'a RefCell<FixedStorage<OpStatus, 5>>>,
    counter: &'a RefCell<u32>,
}

#[sails_rs::service]
impl FixedCounter<'_> {
    #[export(unwrap_result)]
    pub fn request_increment(&mut self) -> Result<MessageId, TrackerError> {
        let id = Syscall::message_id();
        self.tracker.insert(id, OpStatus::Pending)?;
        Ok(id)
    }

    #[export]
    pub fn confirm_increment(&mut self, id: MessageId) -> bool {
        if let Some(OpStatus::Pending) = self.tracker.get_status(&id) {
            *self.counter.borrow_mut() += 1;
            self.tracker.update_status(id, OpStatus::Completed).unwrap();
            return true;
        }
        false
    }

    #[export]
    pub fn get_val(&self) -> u32 {
        *self.counter.borrow()
    }

    #[export]
    pub fn get_status(&self, id: MessageId) -> Option<OpStatus> {
        self.tracker.get_status(&id)
    }

    #[export]
    pub fn get_statuses(&self, query: Option<Pagination>) -> Vec<(MessageId, OpStatus)> {
        self.tracker.get_statuses(query)
    }

    #[export]
    pub fn remove_fixed(&mut self, id: MessageId) -> Option<OpStatus> {
        self.tracker.remove(&id)
    }

    #[export]
    pub fn update_fixed(&mut self, id: MessageId, status: OpStatus) -> bool {
        self.tracker.update_status(id, status).unwrap()
    }

    #[export]
    pub fn clear_fixed(&mut self) {
        self.tracker.clear();
    }
}

#[derive(Default)]
pub struct Program {
    dynamic_tracker_data: RefCell<BTreeMap<MessageId, OpStatus>>,
    fixed_tracker_data: RefCell<FixedStorage<OpStatus, 5>>,
    counter: RefCell<u32>,
}

#[program]
impl Program {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn dynamic_counter(&self) -> DynamicCounter<'_> {
        DynamicCounter {
            tracker: MsgTracker::new(&self.dynamic_tracker_data),
            counter: &self.counter,
        }
    }

    pub fn fixed_counter(&self) -> FixedCounter<'_> {
        FixedCounter {
            tracker: MsgTracker::new(&self.fixed_tracker_data),
            counter: &self.counter,
        }
    }
}
