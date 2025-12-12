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

use awesome_sails_utils::{assert_ok, math::Max};
use awesome_sails_vft_pack::vft::utils::{Allowance, Balance};
use common::{ALICE, BOB, CHARLIE, DAVE, assert_str_panic};
use futures::StreamExt;
use sails_rs::{U256, prelude::*};
use vft_pack_test_client::{
    VftPackTestClient,
    vft::{Vft, events::VftEvents},
    vft_admin::VftAdmin,
    vft_extension::VftExtension,
};

const MAGIC: usize = 21;
const BN: u32 = 137;

#[tokio::test]
async fn allowance() {
    let allowances = vec![(ALICE, BOB, U256::exp10(MAGIC), BN)];
    let balances = Default::default();

    let (program, _env, _pid) =
        common::deploy_with_data(allowances, balances, 0).await;

    let vft_service = program.vft();
    let vft_extension_service = program.vft_extension();

    // # Test case #1.
    // Approve is returned if exists.
    {
        let res = vft_extension_service.allowance_of(ALICE, BOB).await;
        assert_ok!(res, Some((U256::exp10(MAGIC), BN)));

        let res = vft_service.allowance(ALICE, BOB).await;
        assert_ok!(res, U256::exp10(MAGIC));
    }

    // # Test case #2.
    // U256::zero() is returned if not exists.
    {
        let res = vft_extension_service.allowance_of(BOB, ALICE).await;
        assert_ok!(res, None);

        let res = vft_service.allowance(BOB, ALICE).await;
        assert_ok!(res, U256::zero());
    }
}

#[tokio::test]
async fn approve() {
    let (program, _env, pid) =
        common::deploy_with_data(Default::default(), Default::default(), 1).await;
    let mut vft_service = program.vft();
    let vft_extension_service = program.vft_extension();

    let listener_binding = program.vft().listener();
    let mut vft_events = listener_binding.listen().await.unwrap();

    // # Test case #1.
    // Allowance from Alice to Bob doesn't exist and created.
    {
        let res = vft_service.approve(BOB, U256::exp10(MAGIC)).await;
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

        let res = vft_service.allowance(ALICE, BOB).await;
        assert_ok!(res, U256::exp10(MAGIC));
    }

    // # Test case #2.
    // Allowance from Alice to Bob exist and changed (as well as expiry).
    {
        let (_, bn1) = vft_extension_service
            .allowance_of(ALICE, BOB)
            .await
            .expect("infallible")
            .expect("infallible");

        let res = vft_service.approve(BOB, U256::exp10(MAGIC - 1)).await;

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

        let (res, bn2) = vft_extension_service
            .allowance_of(ALICE, BOB)
            .await
            .expect("infallible")
            .expect("infallible");

        assert_eq!(res, U256::exp10(MAGIC - 1));

        assert!(bn2 > bn1);
    }

    // # Test case #3.
    // Allowance from Alice to Bob exists and not changed.
    {
        let (_, bn1) = vft_extension_service
            .allowance_of(ALICE, BOB)
            .await
            .expect("infallible")
            .expect("infallible");

        let res = vft_service.approve(BOB, U256::exp10(MAGIC - 1)).await;

        assert_ok!(res, false);

        let (res, bn2) = vft_extension_service
            .allowance_of(ALICE, BOB)
            .await
            .expect("infallible")
            .expect("infallible");

        assert_eq!(res, U256::exp10(MAGIC - 1));

        assert!(bn2 > bn1);
    }

    // # Test case #4.
    // Allowance from Alice to Bob exists and removed.
    {
        let res = vft_service.approve(BOB, U256::zero()).await;
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

        let res = vft_service.allowance(ALICE, BOB).await;
        assert_ok!(res, U256::zero());

        let res = vft_extension_service.allowance_of(ALICE, BOB).await;
        assert_ok!(res, None);
    }

    // # Test case #5.
    // Allowance from Alice to Bob doesn't exists and not created.
    {
        let res = vft_service.approve(BOB, U256::zero()).await;
        assert_ok!(res, false);

        let res = vft_service.allowance(ALICE, BOB).await;
        assert_ok!(res, U256::zero());
    }

    // # Test case #6.
    // Allowance is always noop on owner == spender.
    {
        let res = vft_service.approve(ALICE, U256::exp10(MAGIC)).await;
        assert_ok!(res, false);

        let res = vft_service.allowance(ALICE, ALICE).await;
        assert_ok!(res, U256::zero());

        let res = vft_service.approve(ALICE, U256::zero()).await;
        assert_ok!(res, false);

        let res = vft_service.allowance(ALICE, ALICE).await;
        assert_ok!(res, U256::zero());
    }
}

#[tokio::test]
async fn balance_of() {
    let allowances = Default::default();
    let balances = vec![(ALICE, U256::exp10(MAGIC))];

    let (program, _env, _pid) =
        common::deploy_with_data(allowances, balances, 0).await;

    let vft_service = program.vft();
    let vft_extension_service = program.vft_extension();

    // # Test case #1.
    // Balance is returned if exists.
    {
        let res = vft_extension_service.balance_of(ALICE).await;
        assert_ok!(res, Some(U256::exp10(MAGIC)));

        let res = vft_service.balance_of(ALICE).await;
        assert_ok!(res, U256::exp10(MAGIC));
    }

    // # Test case #2.
    // U256::zero() is returned if not exists.
    {
        let res = vft_extension_service.balance_of(BOB).await;
        assert_ok!(res, None);

        let res = vft_service.balance_of(BOB).await;
        assert_ok!(res, U256::zero());
    }
}

#[tokio::test]
async fn transfer() {
    let allowances = Default::default();
    let balances = vec![(BOB, U256::exp10(MAGIC)), (DAVE, Balance::MAX.into())];

    let (program, _env, pid) =
        common::deploy_with_data(allowances, balances, 0).await;

    let mut vft_service = program.vft();
    let vft_extension_service = program.vft_extension();

    let listener_binding = program.vft().listener();
    let mut vft_events = listener_binding.listen().await.unwrap();

    // # Test case #1.
    // Alice transfers to Bob, when Alice has no balance.
    {
        let res = vft_service
            .transfer(BOB, U256::exp10(MAGIC - 1))
            .with_actor_id(ALICE)
            .await;

        assert_str_panic(res.unwrap_err(), "insufficient balance");
    }

    // # Test case #2.
    // Bob transfers to Alice, when Bob's balance is less than required.
    {
        let res = vft_service
            .transfer(ALICE, U256::exp10(MAGIC) + U256::one())
            .with_actor_id(BOB)
            .await;

        assert_str_panic(res.unwrap_err(), "insufficient balance");
    }

    // # Test case #3.
    // Dave transfers to Bob, causing numeric overflow.
    {
        let res = vft_service
            .transfer(
                BOB,
                // max - balance_of(bob) + 1
                U256::from(Balance::MAX) - U256::exp10(MAGIC) + U256::one(),
            )
            .with_actor_id(DAVE)
            .await;

        assert_str_panic(res.unwrap_err(), "balance or supply overflow");
    }

    // # Test case #4.
    // Bob transfers to Alice, when Alice's account doesn't exist.
    {
        let res = vft_service
            .transfer(ALICE, U256::exp10(MAGIC - 1))
            .with_actor_id(BOB)
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

        let res = vft_service.balance_of(ALICE).await;
        assert_ok!(res, U256::exp10(MAGIC - 1));

        let res = vft_service.balance_of(BOB).await;
        assert_ok!(res, U256::exp10(MAGIC) - U256::exp10(MAGIC - 1));
    }

    // # Test case #5.
    // Bob transfers to Alice, when Alice's account exists.
    {
        let res = vft_service
            .transfer(ALICE, U256::exp10(MAGIC - 2))
            .with_actor_id(BOB)
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

        let res = vft_service.balance_of(ALICE).await;
        assert_ok!(res, U256::exp10(MAGIC - 1) + U256::exp10(MAGIC - 2));

        let res = vft_service.balance_of(BOB).await;
        assert_ok!(
            res,
            U256::exp10(MAGIC) - U256::exp10(MAGIC - 1) - U256::exp10(MAGIC - 2)
        );
    }

    // # Test case #6.
    // Bob transfers to Alice, when Alice's account exists and Bob's is removed.
    {
        let res = vft_service
            .transfer(
                ALICE,
                U256::exp10(MAGIC) - U256::exp10(MAGIC - 1) - U256::exp10(MAGIC - 2),
            )
            .with_actor_id(BOB)
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

        let res = vft_service.balance_of(ALICE).await;
        assert_ok!(res, U256::exp10(MAGIC));

        let res = vft_extension_service.balance_of(BOB).await;
        assert_ok!(res, None);
    }

    // # Test case #7.
    // Alice transfers to Charlie, when Alice's account is removed and Charlie's is created.
    {
        let res = vft_service
            .transfer(CHARLIE, U256::exp10(MAGIC))
            .with_actor_id(ALICE)
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

        let res = vft_service.balance_of(CHARLIE).await;
        assert_ok!(res, U256::exp10(MAGIC));

        let res = vft_extension_service.balance_of(ALICE).await;
        assert_ok!(res, None);
    }

    // # Test case #8.
    // Transfer is always noop when from == to.
    {
        let res = vft_service
            .transfer(ALICE, U256::exp10(MAGIC))
            .with_actor_id(ALICE)
            .await;

        assert_ok!(res, false);

        let res = vft_extension_service.balance_of(ALICE).await;
        assert_ok!(res, None);

        let res = vft_service
            .transfer(CHARLIE, U256::exp10(MAGIC))
            .with_actor_id(CHARLIE)
            .await;

        assert_ok!(res, false);

        let res = vft_service.balance_of(CHARLIE).await;
        assert_ok!(res, U256::exp10(MAGIC));
    }

    // # Test case #9.
    // Transfer is always noop when value is zero.
    {
        let res = vft_service
            .transfer(CHARLIE, U256::zero())
            .with_actor_id(ALICE)
            .await;

        assert_ok!(res, false);

        let res = vft_service.balance_of(CHARLIE).await;
        assert_ok!(res, U256::exp10(MAGIC));

        let res = vft_service
            .transfer(ALICE, U256::zero())
            .with_actor_id(CHARLIE)
            .await;

        assert_ok!(res, false);

        let res = vft_extension_service.balance_of(ALICE).await;
        assert_ok!(res, None);
    }
}

// Since this uses [`transfer`] in underlying impl, it needs only
// check approval specific logic and few transfer's happy cases.
#[tokio::test]
async fn transfer_from() {
    let allowances = Default::default();
    let balances = vec![(BOB, U256::exp10(MAGIC)), (DAVE, U256::exp10(MAGIC))];

    let (program, _env, pid) =
        common::deploy_with_data(allowances, balances, 1).await;

    let mut vft_service = program.vft();
    let vft_extension_service = program.vft_extension();

    let listener_binding = program.vft().listener();
    let mut vft_events = listener_binding.listen().await.unwrap();

    // # Test case #1.
    // Bob doesn't need approve to transfer from self to Alice.
    // With zero value nothing's changed.
    {
        let res = vft_service
            .transfer_from(BOB, ALICE, U256::zero())
            .with_actor_id(BOB)
            .await;

        assert_ok!(res, false);
    }

    // # Test case #2.
    // Bob doesn't need approve to transfer from self to Alice.
    {
        let res = vft_service
            .transfer_from(BOB, ALICE, U256::exp10(MAGIC))
            .with_actor_id(BOB)
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

        let res = vft_service.balance_of(ALICE).await;
        assert_ok!(res, U256::exp10(MAGIC));

        let res = vft_extension_service.balance_of(BOB).await;
        assert_ok!(res, None);
    }

    // # Test case #3.
    // Noop on self transfer with self approve.
    {
        let res = vft_service
            .transfer_from(BOB, BOB, U256::exp10(MAGIC))
            .with_actor_id(BOB)
            .await;

        assert_ok!(res, false);

        let res = vft_extension_service.balance_of(BOB).await;
        assert_ok!(res, None);

        let res = vft_service
            .transfer_from(ALICE, ALICE, U256::exp10(MAGIC))
            .with_actor_id(ALICE)
            .await;

        assert_ok!(res, false);

        let res = vft_service.balance_of(ALICE).await;
        assert_ok!(res, U256::exp10(MAGIC));
    }

    // # Test case #4.
    // Bob tries to perform transfer from Alice to Charlie with no approval exists.
    {
        let res = vft_service
            .transfer_from(ALICE, CHARLIE, U256::exp10(MAGIC))
            .with_actor_id(BOB)
            .await;

        assert_str_panic(res.unwrap_err(), "insufficient allowance");
    }

    // # Test case #5.
    // Bob tries to perform transfer from Alice to Charlie with insufficient approval.
    {
        let res = vft_service
            .approve(BOB, U256::exp10(MAGIC - 1))
            .with_actor_id(ALICE)
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

        let res = vft_service
            .transfer_from(ALICE, CHARLIE, U256::exp10(MAGIC))
            .with_actor_id(BOB)
            .await;

        assert_str_panic(res.unwrap_err(), "insufficient allowance");
    }

    // # Test case #6.
    // Bob tries to perform transfer from Alice to Charlie with insufficient balance.
    {
        let res = vft_service
            .approve(BOB, Allowance::MAX.into())
            .with_actor_id(ALICE)
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

        let res = vft_service
            .transfer_from(ALICE, CHARLIE, U256::exp10(MAGIC) + U256::one())
            .with_actor_id(BOB)
            .await;

        assert_str_panic(res.unwrap_err(), "insufficient balance");
    }

    // # Test case #7.
    // Bob performs transfer from Alice to Charlie and allowance is unchanged (but expiry changed).
    {
        let (_, bn1) = vft_extension_service
            .allowance_of(ALICE, BOB)
            .await
            .expect("infallible")
            .expect("infallible");

        let res = vft_service
            .transfer_from(ALICE, CHARLIE, U256::exp10(MAGIC - 1))
            .with_actor_id(BOB)
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

        let res = vft_service.balance_of(ALICE).await;
        assert_ok!(res, U256::exp10(MAGIC) - U256::exp10(MAGIC - 1));

        let res = vft_service.balance_of(CHARLIE).await;
        assert_ok!(res, U256::exp10(MAGIC - 1));

        let (res, bn2) = vft_extension_service
            .allowance_of(ALICE, BOB)
            .await
            .expect("infallible")
            .expect("infallible");

        assert_eq!(res, U256::MAX);

        assert!(bn2 > bn1);
    }

    // # Test case #8.
    // Alice performs transfer from Charlie to Dave and allowance is changed.
    {
        let res = vft_service
            .approve(ALICE, U256::exp10(MAGIC - 2))
            .with_actor_id(CHARLIE)
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

        let (_, bn1) = vft_extension_service
            .allowance_of(CHARLIE, ALICE)
            .await
            .expect("infallible")
            .expect("infallible");

        let res = vft_service
            .transfer_from(CHARLIE, DAVE, U256::exp10(MAGIC - 3))
            .with_actor_id(ALICE)
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

        let (res, bn2) = vft_extension_service
            .allowance_of(CHARLIE, ALICE)
            .await
            .expect("infallible")
            .expect("infallible");

        assert_eq!(res, U256::exp10(MAGIC - 2) - U256::exp10(MAGIC - 3));

        assert!(bn2 > bn1);
    }

    // # Test case #9.
    // Alice performs transfer from Charlie to Dave and allowance is removed.
    {
        let res = vft_service
            .transfer_from(
                CHARLIE,
                DAVE,
                U256::exp10(MAGIC - 2) - U256::exp10(MAGIC - 3),
            )
            .with_actor_id(ALICE)
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

        let res = vft_extension_service.allowance_of(CHARLIE, ALICE).await;
        assert_ok!(res, None);

        let res = vft_service.balance_of(CHARLIE).await;
        assert_ok!(res, U256::exp10(MAGIC - 1) - U256::exp10(MAGIC - 2));

        let res = vft_service.balance_of(DAVE).await;
        assert_ok!(res, U256::exp10(MAGIC) + U256::exp10(MAGIC - 2));
    }
}



#[tokio::test]
async fn pause() {
    let allowances = vec![(ALICE, BOB, U256::exp10(MAGIC), BN)];
    let balances = Default::default();

    let (program, _env, _pid) =
        common::deploy_with_data(allowances, balances, 0).await;

    let mut vft_service = program.vft();
    let mut vft_admin_service = program.vft_admin();
    let vft_extension_service = program.vft_extension();

    // Call not paused.
    {
        let res = vft_extension_service.allowance_of(ALICE, BOB).await;
        assert_ok!(res, Some((U256::exp10(MAGIC), BN)));

        let res = vft_service.allowance(ALICE, BOB).await;
        assert_ok!(res, U256::exp10(MAGIC));
    }

    // Pause
    {
        vft_admin_service.pause().await.unwrap();

        let paused = vft_admin_service.is_paused().await.unwrap();
        assert!(paused);
    }

    // Call paused.
    {
        let res = vft_service.transfer(BOB, U256::exp10(10)).await;

        assert_str_panic(res.unwrap_err(), "storage is paused");
    }
}
