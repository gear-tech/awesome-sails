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

mod common;

use awesome_sails::{
    assert_ok,
    math::{Max, OverflowError, UnderflowError},
    pause::PausableError,
};
use awesome_sails_vft_service::utils::{Allowance, AllowancesError, Balance, BalancesError};
use common::{ALICE, BOB, CHARLIE, DAVE, assert_str_panic};
use core::convert::Infallible;
use futures::StreamExt;
use sails_rs::{U256, calls::*, events::Listener};
use test_bin::client::{
    Vft, VftAdmin, VftExtension,
    traits::{Vft as _, VftAdmin as _, VftExtension as _},
    vft::events::{VftEvents, listener as vft_listener},
};

const MAGIC: usize = 21;
const BN: u32 = 137;

#[tokio::test]
async fn allowance() {
    let allowances = vec![(ALICE, BOB, U256::exp10(MAGIC), BN)];
    let balances = Default::default();

    let (remoting, pid) = common::deploy_with_data(allowances, balances, U256::zero(), 0).await;

    let vft = Vft::new(remoting.clone());
    let vft_extension = VftExtension::new(remoting.clone());

    // # Test case #1.
    // Approve is returned if exists.
    {
        let res = vft_extension.allowance_of(ALICE, BOB).recv(pid).await;
        assert_ok!(res, Some((U256::exp10(MAGIC), BN)));

        let res = vft.allowance(ALICE, BOB).recv(pid).await;
        assert_ok!(res, U256::exp10(MAGIC));
    }

    // # Test case #2.
    // U256::zero() is returned if not exists.
    {
        let res = vft_extension.allowance_of(BOB, ALICE).recv(pid).await;
        assert_ok!(res, None);

        let res = vft.allowance(BOB, ALICE).recv(pid).await;
        assert_ok!(res, U256::zero());
    }
}

#[tokio::test]
async fn approve() {
    let (remoting, pid) =
        common::deploy_with_data(Default::default(), Default::default(), U256::zero(), 1).await;
    let remoting = remoting.with_actor_id(ALICE);

    let mut vft = Vft::new(remoting.clone());
    let vft_extension = VftExtension::new(remoting.clone());

    let mut vft_listener = vft_listener(remoting.clone());
    let mut vft_events = vft_listener.listen().await.unwrap();

    // # Test case #1.
    // Allowance from Alice to Bob doesn't exist and created.
    {
        let res = vft.approve(BOB, U256::exp10(MAGIC)).send_recv(pid).await;
        assert_ok!(res, true);

        let (actor, event) = vft_events.next().await.unwrap();
        assert_eq!(actor, pid);
        assert_eq!(
            event,
            VftEvents::Approval {
                owner: ALICE,
                spender: BOB,
                value: U256::exp10(MAGIC),
            }
        );

        let res = vft.allowance(ALICE, BOB).recv(pid).await;
        assert_ok!(res, U256::exp10(MAGIC));
    }

    // # Test case #2.
    // Allowance from Alice to Bob exist and changed (as well as expiry).
    {
        let (_, bn1) = vft_extension
            .allowance_of(ALICE, BOB)
            .recv(pid)
            .await
            .expect("infallible")
            .expect("infallible");

        let res = vft
            .approve(BOB, U256::exp10(MAGIC - 1))
            .send_recv(pid)
            .await;

        assert_ok!(res, true);

        let (actor, event) = vft_events.next().await.unwrap();
        assert_eq!(actor, pid);
        assert_eq!(
            event,
            VftEvents::Approval {
                owner: ALICE,
                spender: BOB,
                value: U256::exp10(MAGIC - 1),
            }
        );

        let (res, bn2) = vft_extension
            .allowance_of(ALICE, BOB)
            .recv(pid)
            .await
            .expect("infallible")
            .expect("infallible");

        assert_eq!(res, U256::exp10(MAGIC - 1));

        assert!(bn2 > bn1);
    }

    // # Test case #3.
    // Allowance from Alice to Bob exists and not changed.
    {
        let (_, bn1) = vft_extension
            .allowance_of(ALICE, BOB)
            .recv(pid)
            .await
            .expect("infallible")
            .expect("infallible");

        let res = vft
            .approve(BOB, U256::exp10(MAGIC - 1))
            .send_recv(pid)
            .await;

        assert_ok!(res, false);

        let (res, bn2) = vft_extension
            .allowance_of(ALICE, BOB)
            .recv(pid)
            .await
            .expect("infallible")
            .expect("infallible");

        assert_eq!(res, U256::exp10(MAGIC - 1));

        assert!(bn2 > bn1);
    }

    // # Test case #4.
    // Allowance from Alice to Bob exists and removed.
    {
        let res = vft.approve(BOB, U256::zero()).send_recv(pid).await;
        assert_ok!(res, true);

        let (actor, event) = vft_events.next().await.unwrap();
        assert_eq!(actor, pid);
        assert_eq!(
            event,
            VftEvents::Approval {
                owner: ALICE,
                spender: BOB,
                value: U256::zero(),
            }
        );

        let res = vft.allowance(ALICE, BOB).recv(pid).await;
        assert_ok!(res, U256::zero());

        let res = vft_extension.allowance_of(ALICE, BOB).recv(pid).await;
        assert_ok!(res, None);
    }

    // # Test case #5.
    // Allowance from Alice to Bob doesn't exists and not created.
    {
        let res = vft.approve(BOB, U256::zero()).send_recv(pid).await;
        assert_ok!(res, false);

        let res = vft.allowance(ALICE, BOB).recv(pid).await;
        assert_ok!(res, U256::zero());
    }

    // # Test case #6.
    // Allowance is always noop on owner == spender.
    {
        let res = vft.approve(ALICE, U256::exp10(MAGIC)).send_recv(pid).await;
        assert_ok!(res, false);

        let res = vft.allowance(ALICE, ALICE).recv(pid).await;
        assert_ok!(res, U256::zero());

        let res = vft.approve(ALICE, U256::zero()).send_recv(pid).await;
        assert_ok!(res, false);

        let res = vft.allowance(ALICE, ALICE).recv(pid).await;
        assert_ok!(res, U256::zero());
    }
}

#[tokio::test]
async fn balance_of() {
    let allowances = Default::default();
    let balances = vec![(ALICE, U256::exp10(MAGIC))];

    let (remoting, pid) = common::deploy_with_data(allowances, balances, U256::zero(), 0).await;

    let vft = Vft::new(remoting.clone());
    let vft_extension = VftExtension::new(remoting.clone());

    // # Test case #1.
    // Balance is returned if exists.
    {
        let res = vft_extension.balance_of(ALICE).recv(pid).await;
        assert_ok!(res, Some(U256::exp10(MAGIC)));

        let res = vft.balance_of(ALICE).recv(pid).await;
        assert_ok!(res, U256::exp10(MAGIC));
    }

    // # Test case #2.
    // U256::zero() is returned if not exists.
    {
        let res = vft_extension.balance_of(BOB).recv(pid).await;
        assert_ok!(res, None);

        let res = vft.balance_of(BOB).recv(pid).await;
        assert_ok!(res, U256::zero());
    }
}

#[tokio::test]
async fn transfer() {
    let allowances = Default::default();
    let balances = vec![(BOB, U256::exp10(MAGIC)), (DAVE, Balance::MAX.into())];

    let (remoting, pid) = common::deploy_with_data(allowances, balances, U256::zero(), 0).await;

    let vft = Vft::new(remoting.clone());
    let vft_extension = VftExtension::new(remoting.clone());

    let mut vft_listener = vft_listener(remoting.clone());
    let mut vft_events = vft_listener.listen().await.unwrap();

    // # Test case #1.
    // Alice transfers to Bob, when Alice has no balance.
    {
        let res = Vft::new(remoting.clone().with_actor_id(ALICE))
            .transfer(BOB, U256::exp10(MAGIC - 1))
            .send_recv(pid)
            .await;

        assert_str_panic(
            res.unwrap_err(),
            BalancesError::Insufficient(UnderflowError),
        );
    }

    // # Test case #2.
    // Bob transfers to Alice, when Bob's balance is less than required.
    {
        let res = Vft::new(remoting.clone().with_actor_id(BOB))
            .transfer(ALICE, U256::exp10(MAGIC) + U256::one())
            .send_recv(pid)
            .await;

        assert_str_panic(
            res.unwrap_err(),
            BalancesError::Insufficient(UnderflowError),
        );
    }

    // # Test case #3.
    // Dave transfers to Bob, causing numeric overflow.
    {
        let res = Vft::new(remoting.clone().with_actor_id(DAVE))
            .transfer(
                BOB,
                // max - balance_of(bob) + 1
                U256::from(Balance::MAX) - U256::exp10(MAGIC) + U256::one(),
            )
            .send_recv(pid)
            .await;

        assert_str_panic(res.unwrap_err(), BalancesError::Overflow(OverflowError));
    }

    // # Test case #4.
    // Bob transfers to Alice, when Alice's account doesn't exist.
    {
        let res = Vft::new(remoting.clone().with_actor_id(BOB))
            .transfer(ALICE, U256::exp10(MAGIC - 1))
            .send_recv(pid)
            .await;

        assert_ok!(res, true);

        let (actor, event) = vft_events.next().await.unwrap();
        assert_eq!(actor, pid);
        assert_eq!(
            event,
            VftEvents::Transfer {
                from: BOB,
                to: ALICE,
                value: U256::exp10(MAGIC - 1),
            }
        );

        let res = vft.balance_of(ALICE).recv(pid).await;
        assert_ok!(res, U256::exp10(MAGIC - 1));

        let res = vft.balance_of(BOB).recv(pid).await;
        assert_ok!(res, U256::exp10(MAGIC) - U256::exp10(MAGIC - 1));
    }

    // # Test case #5.
    // Bob transfers to Alice, when Alice's account exists.
    {
        let res = Vft::new(remoting.clone().with_actor_id(BOB))
            .transfer(ALICE, U256::exp10(MAGIC - 2))
            .send_recv(pid)
            .await;

        assert_ok!(res, true);

        let (actor, event) = vft_events.next().await.unwrap();
        assert_eq!(actor, pid);
        assert_eq!(
            event,
            VftEvents::Transfer {
                from: BOB,
                to: ALICE,
                value: U256::exp10(MAGIC - 2),
            }
        );

        let res = vft.balance_of(ALICE).recv(pid).await;
        assert_ok!(res, U256::exp10(MAGIC - 1) + U256::exp10(MAGIC - 2));

        let res = vft.balance_of(BOB).recv(pid).await;
        assert_ok!(
            res,
            U256::exp10(MAGIC) - U256::exp10(MAGIC - 1) - U256::exp10(MAGIC - 2)
        );
    }

    // # Test case #6.
    // Bob transfers to Alice, when Alice's account exists and Bob's is removed.
    {
        let res = Vft::new(remoting.clone().with_actor_id(BOB))
            .transfer(
                ALICE,
                U256::exp10(MAGIC) - U256::exp10(MAGIC - 1) - U256::exp10(MAGIC - 2),
            )
            .send_recv(pid)
            .await;

        assert_ok!(res, true);

        let (actor, event) = vft_events.next().await.unwrap();
        assert_eq!(actor, pid);
        assert_eq!(
            event,
            VftEvents::Transfer {
                from: BOB,
                to: ALICE,
                value: U256::exp10(MAGIC) - U256::exp10(MAGIC - 1) - U256::exp10(MAGIC - 2),
            }
        );

        let res = vft.balance_of(ALICE).recv(pid).await;
        assert_ok!(res, U256::exp10(MAGIC));

        let res = vft_extension.balance_of(BOB).recv(pid).await;
        assert_ok!(res, None);
    }

    // # Test case #7.
    // Alice transfers to Charlie, when Alice's account is removed and Charlie's is created.
    {
        let res = Vft::new(remoting.clone().with_actor_id(ALICE))
            .transfer(CHARLIE, U256::exp10(MAGIC))
            .send_recv(pid)
            .await;

        assert_ok!(res, true);

        let (actor, event) = vft_events.next().await.unwrap();
        assert_eq!(actor, pid);
        assert_eq!(
            event,
            VftEvents::Transfer {
                from: ALICE,
                to: CHARLIE,
                value: U256::exp10(MAGIC),
            }
        );

        let res = vft.balance_of(CHARLIE).recv(pid).await;
        assert_ok!(res, U256::exp10(MAGIC));

        let res = vft_extension.balance_of(ALICE).recv(pid).await;
        assert_ok!(res, None);
    }

    // # Test case #8.
    // Transfer is always noop when from == to.
    {
        let res = Vft::new(remoting.clone().with_actor_id(ALICE))
            .transfer(ALICE, U256::exp10(MAGIC))
            .send_recv(pid)
            .await;

        assert_ok!(res, false);

        let res = vft_extension.balance_of(ALICE).recv(pid).await;
        assert_ok!(res, None);

        let res = Vft::new(remoting.clone().with_actor_id(CHARLIE))
            .transfer(CHARLIE, U256::exp10(MAGIC))
            .send_recv(pid)
            .await;

        assert_ok!(res, false);

        let res = vft.balance_of(CHARLIE).recv(pid).await;
        assert_ok!(res, U256::exp10(MAGIC));
    }

    // # Test case #9.
    // Transfer is always noop when value is zero.
    {
        let res = Vft::new(remoting.clone().with_actor_id(ALICE))
            .transfer(CHARLIE, U256::zero())
            .send_recv(pid)
            .await;

        assert_ok!(res, false);

        let res = vft.balance_of(CHARLIE).recv(pid).await;
        assert_ok!(res, U256::exp10(MAGIC));

        let res = Vft::new(remoting.clone().with_actor_id(CHARLIE))
            .transfer(ALICE, U256::zero())
            .send_recv(pid)
            .await;

        assert_ok!(res, false);

        let res = vft_extension.balance_of(ALICE).recv(pid).await;
        assert_ok!(res, None);
    }
}

// Since this uses [`transfer`] in underlying impl, it needs only
// check approval specific logic and few transfer's happy cases.
#[tokio::test]
async fn transfer_from() {
    let allowances = Default::default();
    let balances = vec![(BOB, U256::exp10(MAGIC)), (DAVE, U256::exp10(MAGIC))];

    let (remoting, pid) = common::deploy_with_data(allowances, balances, U256::zero(), 1).await;

    let vft = Vft::new(remoting.clone());
    let vft_extension = VftExtension::new(remoting.clone());

    let mut vft_listener = vft_listener(remoting.clone());
    let mut vft_events = vft_listener.listen().await.unwrap();

    // # Test case #1.
    // Bob doesn't need approve to transfer from self to Alice.
    // With zero value nothing's changed.
    {
        let res = Vft::new(remoting.clone().with_actor_id(BOB))
            .transfer_from(BOB, ALICE, U256::zero())
            .send_recv(pid)
            .await;

        assert_ok!(res, false);
    }

    // # Test case #2.
    // Bob doesn't need approve to transfer from self to Alice.
    {
        let res = Vft::new(remoting.clone().with_actor_id(BOB))
            .transfer_from(BOB, ALICE, U256::exp10(MAGIC))
            .send_recv(pid)
            .await;

        assert_ok!(res, true);

        let (actor, event) = vft_events.next().await.unwrap();
        assert_eq!(actor, pid);
        assert_eq!(
            event,
            VftEvents::Transfer {
                from: BOB,
                to: ALICE,
                value: U256::exp10(MAGIC),
            }
        );

        let res = vft.balance_of(ALICE).recv(pid).await;
        assert_ok!(res, U256::exp10(MAGIC));

        let res = vft_extension.balance_of(BOB).recv(pid).await;
        assert_ok!(res, None);
    }

    // # Test case #3.
    // Noop on self transfer with self approve.
    {
        let res = Vft::new(remoting.clone().with_actor_id(BOB))
            .transfer_from(BOB, BOB, U256::exp10(MAGIC))
            .send_recv(pid)
            .await;

        assert_ok!(res, false);

        let res = vft_extension.balance_of(BOB).recv(pid).await;
        assert_ok!(res, None);

        let res = Vft::new(remoting.clone().with_actor_id(ALICE))
            .transfer_from(ALICE, ALICE, U256::exp10(MAGIC))
            .send_recv(pid)
            .await;

        assert_ok!(res, false);

        let res = vft.balance_of(ALICE).recv(pid).await;
        assert_ok!(res, U256::exp10(MAGIC));
    }

    // # Test case #4.
    // Bob tries to perform transfer from Alice to Charlie with no approval exists.
    {
        let res = Vft::new(remoting.clone().with_actor_id(BOB))
            .transfer_from(ALICE, CHARLIE, U256::exp10(MAGIC))
            .send_recv(pid)
            .await;

        assert_str_panic(
            res.unwrap_err(),
            AllowancesError::Insufficient(UnderflowError),
        );
    }

    // # Test case #5.
    // Bob tries to perform transfer from Alice to Charlie with insufficient approval.
    {
        let res = Vft::new(remoting.clone().with_actor_id(ALICE))
            .approve(BOB, U256::exp10(MAGIC - 1))
            .send_recv(pid)
            .await;

        assert_ok!(res, true);

        let (actor, event) = vft_events.next().await.unwrap();
        assert_eq!(actor, pid);
        assert_eq!(
            event,
            VftEvents::Approval {
                owner: ALICE,
                spender: BOB,
                value: U256::exp10(MAGIC - 1),
            }
        );

        let res = Vft::new(remoting.clone().with_actor_id(BOB))
            .transfer_from(ALICE, CHARLIE, U256::exp10(MAGIC))
            .send_recv(pid)
            .await;

        assert_str_panic(
            res.unwrap_err(),
            AllowancesError::Insufficient(UnderflowError),
        );
    }

    // # Test case #6.
    // Bob tries to perform transfer from Alice to Charlie with insufficient balance.
    {
        let res = Vft::new(remoting.clone().with_actor_id(ALICE))
            .approve(BOB, Allowance::MAX.into())
            .send_recv(pid)
            .await;

        assert_ok!(res, true);

        let (actor, event) = vft_events.next().await.unwrap();
        assert_eq!(actor, pid);
        assert_eq!(
            event,
            VftEvents::Approval {
                owner: ALICE,
                spender: BOB,
                value: U256::MAX,
            }
        );

        let res = Vft::new(remoting.clone().with_actor_id(BOB))
            .transfer_from(ALICE, CHARLIE, U256::exp10(MAGIC) + U256::one())
            .send_recv(pid)
            .await;

        assert_str_panic(
            res.unwrap_err(),
            BalancesError::Insufficient(UnderflowError),
        );
    }

    // # Test case #7.
    // Bob performs transfer from Alice to Charlie and allowance is unchanged (but expiry changed).
    {
        let (_, bn1) = vft_extension
            .allowance_of(ALICE, BOB)
            .recv(pid)
            .await
            .expect("infallible")
            .expect("infallible");

        let res = Vft::new(remoting.clone().with_actor_id(BOB))
            .transfer_from(ALICE, CHARLIE, U256::exp10(MAGIC - 1))
            .send_recv(pid)
            .await;

        assert_ok!(res, true);

        let (actor, event) = vft_events.next().await.unwrap();
        assert_eq!(actor, pid);
        assert_eq!(
            event,
            VftEvents::Transfer {
                from: ALICE,
                to: CHARLIE,
                value: U256::exp10(MAGIC - 1),
            }
        );

        let res = vft.balance_of(ALICE).recv(pid).await;
        assert_ok!(res, U256::exp10(MAGIC) - U256::exp10(MAGIC - 1));

        let res = vft.balance_of(CHARLIE).recv(pid).await;
        assert_ok!(res, U256::exp10(MAGIC - 1));

        let (res, bn2) = vft_extension
            .allowance_of(ALICE, BOB)
            .recv(pid)
            .await
            .expect("infallible")
            .expect("infallible");

        assert_eq!(res, U256::MAX);

        assert!(bn2 > bn1);
    }

    // # Test case #8.
    // Alice performs transfer from Charlie to Dave and allowance is changed.
    {
        let res = Vft::new(remoting.clone().with_actor_id(CHARLIE))
            .approve(ALICE, U256::exp10(MAGIC - 2))
            .send_recv(pid)
            .await;

        assert_ok!(res, true);

        let (actor, event) = vft_events.next().await.unwrap();
        assert_eq!(actor, pid);
        assert_eq!(
            event,
            VftEvents::Approval {
                owner: CHARLIE,
                spender: ALICE,
                value: U256::exp10(MAGIC - 2),
            }
        );

        let (_, bn1) = vft_extension
            .allowance_of(CHARLIE, ALICE)
            .recv(pid)
            .await
            .expect("infallible")
            .expect("infallible");

        let res = Vft::new(remoting.clone().with_actor_id(ALICE))
            .transfer_from(CHARLIE, DAVE, U256::exp10(MAGIC - 3))
            .send_recv(pid)
            .await;

        assert_ok!(res, true);

        let (actor, event) = vft_events.next().await.unwrap();
        assert_eq!(actor, pid);
        assert_eq!(
            event,
            VftEvents::Transfer {
                from: CHARLIE,
                to: DAVE,
                value: U256::exp10(MAGIC - 3),
            }
        );

        let (res, bn2) = vft_extension
            .allowance_of(CHARLIE, ALICE)
            .recv(pid)
            .await
            .expect("infallible")
            .expect("infallible");

        assert_eq!(res, U256::exp10(MAGIC - 2) - U256::exp10(MAGIC - 3));

        assert!(bn2 > bn1);
    }

    // # Test case #9.
    // Alice performs transfer from Charlie to Dave and allowance is removed.
    {
        let res = Vft::new(remoting.clone().with_actor_id(ALICE))
            .transfer_from(
                CHARLIE,
                DAVE,
                U256::exp10(MAGIC - 2) - U256::exp10(MAGIC - 3),
            )
            .send_recv(pid)
            .await;

        assert_ok!(res, true);

        let (actor, event) = vft_events.next().await.unwrap();
        assert_eq!(actor, pid);
        assert_eq!(
            event,
            VftEvents::Transfer {
                from: CHARLIE,
                to: DAVE,
                value: U256::exp10(MAGIC - 2) - U256::exp10(MAGIC - 3),
            }
        );

        let res = vft_extension.allowance_of(CHARLIE, ALICE).recv(pid).await;
        assert_ok!(res, None);

        let res = vft.balance_of(CHARLIE).recv(pid).await;
        assert_ok!(res, U256::exp10(MAGIC - 1) - U256::exp10(MAGIC - 2));

        let res = vft.balance_of(DAVE).recv(pid).await;
        assert_ok!(res, U256::exp10(MAGIC) + U256::exp10(MAGIC - 2));
    }
}

#[tokio::test]
async fn minimum_balance() {
    let allowances = Default::default();
    let balances = vec![(ALICE, U256::exp10(MAGIC)), (DAVE, U256::exp10(MAGIC))];

    // AKA ED (Existential Deposit).
    let minimum_balance = U256::exp10(10);
    let below_minimum = minimum_balance - U256::one();

    let (remoting, pid) = common::deploy_with_data(allowances, balances, minimum_balance, 1).await;

    let mut vft = Vft::new(remoting.clone());
    let vft_extension = VftExtension::new(remoting.clone());

    let mut vft_listener = vft_listener(remoting.clone());
    let mut vft_events = vft_listener.listen().await.unwrap();

    // # Test case #0.
    // Assert deploy parameters.
    {
        let res = vft.total_supply().recv(pid).await;
        assert_ok!(res, U256::exp10(MAGIC) + U256::exp10(MAGIC));

        let res = vft_extension.unused_value().recv(pid).await;
        assert_ok!(res, U256::zero());

        let res = vft_extension.minimum_balance().recv(pid).await;
        assert_ok!(res, minimum_balance);
    }

    // # Test case #1.
    // Alice transfers to Bob value below ED: Bob cannot receive it.
    {
        let res = vft.transfer(BOB, below_minimum).send_recv(pid).await;

        assert_str_panic(res.unwrap_err(), BalancesError::BelowMinimum);
    }

    // # Test case #2.
    // Alice transfers to Dave value below ED: Dave can receive it.
    {
        let res = vft.transfer(DAVE, below_minimum).send_recv(pid).await;

        assert_ok!(res, true);

        let (actor, event) = vft_events.next().await.unwrap();
        assert_eq!(actor, pid);
        assert_eq!(
            event,
            VftEvents::Transfer {
                from: ALICE,
                to: DAVE,
                value: below_minimum,
            }
        );
    }

    // # Test case #3.
    // Dave transfers to Alice value and goes below ED: remaining are burnt from Dave to unused.
    {
        let res = Vft::new(remoting.clone().with_actor_id(DAVE))
            .transfer(ALICE, U256::exp10(MAGIC))
            .send_recv(pid)
            .await;

        assert_ok!(res, true);

        let (actor, event) = vft_events.next().await.unwrap();
        assert_eq!(actor, pid);
        assert_eq!(
            event,
            VftEvents::Transfer {
                from: DAVE,
                to: ALICE,
                value: U256::exp10(MAGIC),
            }
        );

        let res = vft.balance_of(ALICE).recv(pid).await;
        assert_ok!(res, U256::exp10(MAGIC) + U256::exp10(MAGIC) - below_minimum);

        let res = vft_extension.balance_of(DAVE).recv(pid).await;
        assert_ok!(res, None);

        let res = vft_extension.unused_value().recv(pid).await;
        assert_ok!(res, below_minimum);

        let res = vft.total_supply().recv(pid).await;
        assert_ok!(res, U256::exp10(MAGIC) + U256::exp10(MAGIC));
    }

    // # Test case #4.
    // Bob transfers from Alice to Charlie value below ED: Charlie cannot receive it.
    {
        let res = vft
            .approve(BOB, U256::MAX - U256::one())
            .send_recv(pid)
            .await;
        assert_ok!(res, true);

        let (actor, event) = vft_events.next().await.unwrap();
        assert_eq!(actor, pid);
        assert_eq!(
            event,
            VftEvents::Approval {
                owner: ALICE,
                spender: BOB,
                value: U256::MAX,
            }
        );

        let res = Vft::new(remoting.clone().with_actor_id(BOB))
            .transfer_from(ALICE, CHARLIE, below_minimum)
            .send_recv(pid)
            .await;

        assert_str_panic(res.unwrap_err(), BalancesError::BelowMinimum);
    }

    // # Test case #5.
    // Bob transfers from Alice to Charlie and burns Alice.
    {
        let res = Vft::new(remoting.clone().with_actor_id(BOB))
            .transfer_from(
                ALICE,
                CHARLIE,
                U256::exp10(MAGIC) + U256::exp10(MAGIC) - below_minimum - below_minimum,
            )
            .send_recv(pid)
            .await;

        assert_ok!(res, true);

        let (actor, event) = vft_events.next().await.unwrap();
        assert_eq!(actor, pid);
        assert_eq!(
            event,
            VftEvents::Transfer {
                from: ALICE,
                to: CHARLIE,
                value: U256::exp10(MAGIC) + U256::exp10(MAGIC) - below_minimum - below_minimum,
            }
        );

        let res = vft_extension.balance_of(ALICE).recv(pid).await;
        assert_ok!(res, None);

        let res = vft_extension.unused_value().recv(pid).await;
        assert_ok!(res, below_minimum + below_minimum);

        let res = vft.total_supply().recv(pid).await;
        assert_ok!(res, U256::exp10(MAGIC) + U256::exp10(MAGIC));
    }
}

#[tokio::test]
async fn pause() {
    let allowances = vec![(ALICE, BOB, U256::exp10(MAGIC), BN)];
    let balances = Default::default();

    let (remoting, pid) = common::deploy_with_data(allowances, balances, U256::zero(), 0).await;

    let mut vft = Vft::new(remoting.clone());
    let mut vft_admin = VftAdmin::new(remoting.clone());
    let vft_extension = VftExtension::new(remoting.clone());

    // Call not paused.
    {
        let res = vft_extension.allowance_of(ALICE, BOB).recv(pid).await;
        assert_ok!(res, Some((U256::exp10(MAGIC), BN)));

        let res = vft.allowance(ALICE, BOB).recv(pid).await;
        assert_ok!(res, U256::exp10(MAGIC));
    }

    // Pause
    {
        vft_admin.pause().send_recv(pid).await.unwrap();

        let paused = vft_admin.is_paused().recv(pid).await.unwrap();
        assert!(paused);
    }

    // Call paused.
    {
        let res = vft.transfer(BOB, U256::exp10(10)).send_recv(pid).await;

        assert_str_panic(res.unwrap_err(), PausableError::<Infallible>::Paused);
    }
}
