# Awesome Sails Message Tracker

> **Note:** Built using the Sails framework. It is highly recommended to study the [Sails Documentation](https://docs.rs/sails-rs/latest/sails_rs/) before using this crate.

A library for tracking asynchronous messages and their statuses in Sails-based programs. This crate provides a mechanism to manage the state of outgoing or incoming messages, which is particularly useful for implementing complex asynchronous flows like the Saga pattern.

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
awesome-sails-msg-tracker = "x.y.z"
```

## Usage

### On-Chain: Integration

To use the Message Tracker in your Sails program, you need to include its storage in your program struct and use it within your services.

```rust
#![no_std]

use awesome_sails::msg_tracker::{MsgTracker, storage::BTreeMap};
use awesome_sails_storage::StorageRefCell;
use sails_rs::{cell::RefCell, prelude::*};

#[derive(Clone, Encode, Decode, TypeInfo, PartialEq, Debug)]
pub enum OpStatus {
    Pending,
    Completed,
}

#[derive(Default)]
pub struct Program {
    tracker_data: RefCell<BTreeMap<MessageId, OpStatus>>,
}

#[program]
impl Program {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn my_service(&self) -> MyService<'_> {
        MyService {
            tracker: MsgTracker::new(StorageRefCell::new(&self.tracker_data)),
        }
    }
}

pub struct MyService<'a> {
    tracker: MsgTracker<OpStatus, StorageRefCell<'a, BTreeMap<MessageId, OpStatus>>>,
}

#[service]
impl MyService<'_> {
    pub fn do_something(&mut self) {
        let msg_id = Syscall::message_id();
        self.tracker.insert(msg_id, OpStatus::Pending).expect("Storage full");
    }

    pub fn get_status(&self, id: MessageId) -> Option<OpStatus> {
        self.tracker.get_status(&id)
    }
}
```
