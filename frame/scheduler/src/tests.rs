// This file is part of Substrate.

// Copyright (C) 2017-2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # Scheduler tests.

use super::*;
use crate::mock::{logger, new_test_ext, root, run_to_block, Call, LoggerCall, Scheduler, Test, *};
use frame_support::{
	assert_err, assert_noop, assert_ok,
	traits::{Contains, GetStorageVersion, OnInitialize, QueryPreimage, StorePreimage},
	Hashable,
};
use sp_runtime::traits::Hash;
use substrate_test_utils::assert_eq_uvec;

#[test]
fn basic_scheduling_works() {
	new_test_ext().execute_with(|| {
		let call = Call::Logger(LoggerCall::log { i: 42, weight: Weight::from_ref_time(10) });
		assert!(!<Test as frame_system::Config>::BaseCallFilter::contains(&call));
		assert_ok!(Scheduler::do_schedule(
			DispatchTime::At(4),
			None,
			127,
			root(),
			Preimage::bound(call).unwrap()
		));
		run_to_block(3);
		assert!(logger::log().is_empty());
		run_to_block(4);
		assert_eq!(logger::log(), vec![(root(), 42u32)]);
		run_to_block(100);
		assert_eq!(logger::log(), vec![(root(), 42u32)]);
	});
}

#[test]
fn scheduling_with_preimages_works() {
	new_test_ext().execute_with(|| {
		let call = Call::Logger(LoggerCall::log { i: 42, weight: Weight::from_ref_time(10) });
		let hash = <Test as frame_system::Config>::Hashing::hash_of(&call);
		let len = call.using_encoded(|x| x.len()) as u32;
		let hashed = Preimage::pick(hash, len);
		assert_ok!(Preimage::note_preimage(Origin::signed(0), call.encode()));
		assert_ok!(Scheduler::do_schedule(DispatchTime::At(4), None, 127, root(), hashed));
		assert!(Preimage::is_requested(&hash));
		run_to_block(3);
		assert!(logger::log().is_empty());
		run_to_block(4);
		assert!(!Preimage::len(&hash).is_some());
		assert!(!Preimage::is_requested(&hash));
		assert_eq!(logger::log(), vec![(root(), 42u32)]);
		run_to_block(100);
		assert_eq!(logger::log(), vec![(root(), 42u32)]);
	});
}

#[test]
fn schedule_after_works() {
	new_test_ext().execute_with(|| {
		run_to_block(2);
		let call = Call::Logger(LoggerCall::log { i: 42, weight: Weight::from_ref_time(10) });
		assert!(!<Test as frame_system::Config>::BaseCallFilter::contains(&call));
		// This will schedule the call 3 blocks after the next block... so block 3 + 3 = 6
		assert_ok!(Scheduler::do_schedule(
			DispatchTime::After(3),
			None,
			127,
			root(),
			Preimage::bound(call).unwrap()
		));
		run_to_block(5);
		assert!(logger::log().is_empty());
		run_to_block(6);
		assert_eq!(logger::log(), vec![(root(), 42u32)]);
		run_to_block(100);
		assert_eq!(logger::log(), vec![(root(), 42u32)]);
	});
}

#[test]
fn schedule_after_zero_works() {
	new_test_ext().execute_with(|| {
		run_to_block(2);
		let call = Call::Logger(LoggerCall::log { i: 42, weight: Weight::from_ref_time(10) });
		assert!(!<Test as frame_system::Config>::BaseCallFilter::contains(&call));
		assert_ok!(Scheduler::do_schedule(
			DispatchTime::After(0),
			None,
			127,
			root(),
			Preimage::bound(call).unwrap()
		));
		// Will trigger on the next block.
		run_to_block(3);
		assert_eq!(logger::log(), vec![(root(), 42u32)]);
		run_to_block(100);
		assert_eq!(logger::log(), vec![(root(), 42u32)]);
	});
}

#[test]
fn periodic_scheduling_works() {
	new_test_ext().execute_with(|| {
		// at #4, every 3 blocks, 3 times.
		assert_ok!(Scheduler::do_schedule(
			DispatchTime::At(4),
			Some((3, 3)),
			127,
			root(),
			Preimage::bound(Call::Logger(logger::Call::log {
				i: 42,
				weight: Weight::from_ref_time(10)
			}))
			.unwrap()
		));
		run_to_block(3);
		assert!(logger::log().is_empty());
		run_to_block(4);
		assert_eq!(logger::log(), vec![(root(), 42u32)]);
		run_to_block(6);
		assert_eq!(logger::log(), vec![(root(), 42u32)]);
		run_to_block(7);
		assert_eq!(logger::log(), vec![(root(), 42u32), (root(), 42u32)]);
		run_to_block(9);
		assert_eq!(logger::log(), vec![(root(), 42u32), (root(), 42u32)]);
		run_to_block(10);
		assert_eq!(logger::log(), vec![(root(), 42u32), (root(), 42u32), (root(), 42u32)]);
		run_to_block(100);
		assert_eq!(logger::log(), vec![(root(), 42u32), (root(), 42u32), (root(), 42u32)]);
	});
}

#[test]
fn reschedule_works() {
	new_test_ext().execute_with(|| {
		let call = Call::Logger(LoggerCall::log { i: 42, weight: Weight::from_ref_time(10) });
		assert!(!<Test as frame_system::Config>::BaseCallFilter::contains(&call));
		assert_eq!(
			Scheduler::do_schedule(
				DispatchTime::At(4),
				None,
				127,
				root(),
				Preimage::bound(call).unwrap()
			)
			.unwrap(),
			(4, 0)
		);

		run_to_block(3);
		assert!(logger::log().is_empty());

		assert_eq!(Scheduler::do_reschedule((4, 0), DispatchTime::At(6)).unwrap(), (6, 0));

		assert_noop!(
			Scheduler::do_reschedule((6, 0), DispatchTime::At(6)),
			Error::<Test>::RescheduleNoChange
		);

		run_to_block(4);
		assert!(logger::log().is_empty());

		run_to_block(6);
		assert_eq!(logger::log(), vec![(root(), 42u32)]);

		run_to_block(100);
		assert_eq!(logger::log(), vec![(root(), 42u32)]);
	});
}

#[test]
fn reschedule_named_works() {
	new_test_ext().execute_with(|| {
		let call = Call::Logger(LoggerCall::log { i: 42, weight: Weight::from_ref_time(10) });
		assert!(!<Test as frame_system::Config>::BaseCallFilter::contains(&call));
		assert_eq!(
			Scheduler::do_schedule_named(
				[1u8; 32],
				DispatchTime::At(4),
				None,
				127,
				root(),
				Preimage::bound(call).unwrap(),
			)
			.unwrap(),
			(4, 0)
		);

		run_to_block(3);
		assert!(logger::log().is_empty());

		assert_eq!(Scheduler::do_reschedule_named([1u8; 32], DispatchTime::At(6)).unwrap(), (6, 0));

		assert_noop!(
			Scheduler::do_reschedule_named([1u8; 32], DispatchTime::At(6)),
			Error::<Test>::RescheduleNoChange
		);

		run_to_block(4);
		assert!(logger::log().is_empty());

		run_to_block(6);
		assert_eq!(logger::log(), vec![(root(), 42u32)]);

		run_to_block(100);
		assert_eq!(logger::log(), vec![(root(), 42u32)]);
	});
}

#[test]
fn reschedule_named_perodic_works() {
	new_test_ext().execute_with(|| {
		let call = Call::Logger(LoggerCall::log { i: 42, weight: Weight::from_ref_time(10) });
		assert!(!<Test as frame_system::Config>::BaseCallFilter::contains(&call));
		assert_eq!(
			Scheduler::do_schedule_named(
				[1u8; 32],
				DispatchTime::At(4),
				Some((3, 3)),
				127,
				root(),
				Preimage::bound(call).unwrap(),
			)
			.unwrap(),
			(4, 0)
		);

		run_to_block(3);
		assert!(logger::log().is_empty());

		assert_eq!(Scheduler::do_reschedule_named([1u8; 32], DispatchTime::At(5)).unwrap(), (5, 0));
		assert_eq!(Scheduler::do_reschedule_named([1u8; 32], DispatchTime::At(6)).unwrap(), (6, 0));

		run_to_block(5);
		assert!(logger::log().is_empty());

		run_to_block(6);
		assert_eq!(logger::log(), vec![(root(), 42u32)]);

		assert_eq!(
			Scheduler::do_reschedule_named([1u8; 32], DispatchTime::At(10)).unwrap(),
			(10, 0)
		);

		run_to_block(9);
		assert_eq!(logger::log(), vec![(root(), 42u32)]);

		run_to_block(10);
		assert_eq!(logger::log(), vec![(root(), 42u32), (root(), 42u32)]);

		run_to_block(13);
		assert_eq!(logger::log(), vec![(root(), 42u32), (root(), 42u32), (root(), 42u32)]);

		run_to_block(100);
		assert_eq!(logger::log(), vec![(root(), 42u32), (root(), 42u32), (root(), 42u32)]);
	});
}

#[test]
fn cancel_named_scheduling_works_with_normal_cancel() {
	new_test_ext().execute_with(|| {
		// at #4.
		Scheduler::do_schedule_named(
			[1u8; 32],
			DispatchTime::At(4),
			None,
			127,
			root(),
			Preimage::bound(Call::Logger(LoggerCall::log {
				i: 69,
				weight: Weight::from_ref_time(10),
			}))
			.unwrap(),
		)
		.unwrap();
		let i = Scheduler::do_schedule(
			DispatchTime::At(4),
			None,
			127,
			root(),
			Preimage::bound(Call::Logger(LoggerCall::log {
				i: 42,
				weight: Weight::from_ref_time(10),
			}))
			.unwrap(),
		)
		.unwrap();
		run_to_block(3);
		assert!(logger::log().is_empty());
		assert_ok!(Scheduler::do_cancel_named(None, [1u8; 32]));
		assert_ok!(Scheduler::do_cancel(None, i));
		run_to_block(100);
		assert!(logger::log().is_empty());
	});
}

#[test]
fn cancel_named_periodic_scheduling_works() {
	new_test_ext().execute_with(|| {
		// at #4, every 3 blocks, 3 times.
		Scheduler::do_schedule_named(
			[1u8; 32],
			DispatchTime::At(4),
			Some((3, 3)),
			127,
			root(),
			Preimage::bound(Call::Logger(LoggerCall::log {
				i: 42,
				weight: Weight::from_ref_time(10),
			}))
			.unwrap(),
		)
		.unwrap();
		// same id results in error.
		assert!(Scheduler::do_schedule_named(
			[1u8; 32],
			DispatchTime::At(4),
			None,
			127,
			root(),
			Preimage::bound(Call::Logger(LoggerCall::log {
				i: 69,
				weight: Weight::from_ref_time(10)
			}))
			.unwrap(),
		)
		.is_err());
		// different id is ok.
		Scheduler::do_schedule_named(
			[2u8; 32],
			DispatchTime::At(8),
			None,
			127,
			root(),
			Preimage::bound(Call::Logger(LoggerCall::log {
				i: 69,
				weight: Weight::from_ref_time(10),
			}))
			.unwrap(),
		)
		.unwrap();
		run_to_block(3);
		assert!(logger::log().is_empty());
		run_to_block(4);
		assert_eq!(logger::log(), vec![(root(), 42u32)]);
		run_to_block(6);
		assert_ok!(Scheduler::do_cancel_named(None, [1u8; 32]));
		run_to_block(100);
		assert_eq!(logger::log(), vec![(root(), 42u32), (root(), 69u32)]);
	});
}

#[test]
fn scheduler_respects_weight_limits() {
	let max_weight: Weight = <Test as Config>::MaximumWeight::get();
	new_test_ext().execute_with(|| {
		let call = Call::Logger(LoggerCall::log { i: 42, weight: max_weight / 3 * 2 });
		assert_ok!(Scheduler::do_schedule(
			DispatchTime::At(4),
			None,
			127,
			root(),
			Preimage::bound(call).unwrap(),
		));
		let call = Call::Logger(LoggerCall::log { i: 69, weight: max_weight / 3 * 2 });
		assert_ok!(Scheduler::do_schedule(
			DispatchTime::At(4),
			None,
			127,
			root(),
			Preimage::bound(call).unwrap(),
		));
		// 69 and 42 do not fit together
		run_to_block(4);
		assert_eq!(logger::log(), vec![(root(), 42u32)]);
		run_to_block(5);
		assert_eq!(logger::log(), vec![(root(), 42u32), (root(), 69u32)]);
	});
}

#[test]
fn scheduler_respects_priority_ordering() {
	let max_weight: Weight = <Test as Config>::MaximumWeight::get();
	new_test_ext().execute_with(|| {
		let call = Call::Logger(LoggerCall::log { i: 42, weight: max_weight / 3 });
		assert_ok!(Scheduler::do_schedule(
			DispatchTime::At(4),
			None,
			1,
			root(),
			Preimage::bound(call).unwrap(),
		));
		let call = Call::Logger(LoggerCall::log { i: 69, weight: max_weight / 3 });
		assert_ok!(Scheduler::do_schedule(
			DispatchTime::At(4),
			None,
			0,
			root(),
			Preimage::bound(call).unwrap(),
		));
		run_to_block(4);
		assert_eq!(logger::log(), vec![(root(), 69u32), (root(), 42u32)]);
	});
}

#[test]
fn scheduler_respects_priority_ordering_with_soft_deadlines() {
	new_test_ext().execute_with(|| {
		let max_weight: Weight = <Test as Config>::MaximumWeight::get();
		let call = Call::Logger(LoggerCall::log { i: 42, weight: max_weight / 5 * 2 });
		assert_ok!(Scheduler::do_schedule(
			DispatchTime::At(4),
			None,
			255,
			root(),
			Preimage::bound(call).unwrap(),
		));
		let call = Call::Logger(LoggerCall::log { i: 69, weight: max_weight / 5 * 2 });
		assert_ok!(Scheduler::do_schedule(
			DispatchTime::At(4),
			None,
			127,
			root(),
			Preimage::bound(call).unwrap(),
		));
		let call = Call::Logger(LoggerCall::log { i: 2600, weight: max_weight / 5 * 4 });
		assert_ok!(Scheduler::do_schedule(
			DispatchTime::At(4),
			None,
			126,
			root(),
			Preimage::bound(call).unwrap(),
		));

		// 2600 does not fit with 69 or 42, but has higher priority, so will go through
		run_to_block(4);
		assert_eq!(logger::log(), vec![(root(), 2600u32)]);
		// 69 and 42 fit together
		run_to_block(5);
		assert_eq!(logger::log(), vec![(root(), 2600u32), (root(), 69u32), (root(), 42u32)]);
	});
}

#[test]
fn on_initialize_weight_is_correct() {
	new_test_ext().execute_with(|| {
		let call_weight = Weight::from_ref_time(25);

		// Named
		let call =
			Call::Logger(LoggerCall::log { i: 3, weight: call_weight + Weight::from_ref_time(1) });
		assert_ok!(Scheduler::do_schedule_named(
			[1u8; 32],
			DispatchTime::At(3),
			None,
			255,
			root(),
			Preimage::bound(call).unwrap(),
		));
		let call =
			Call::Logger(LoggerCall::log { i: 42, weight: call_weight + Weight::from_ref_time(2) });
		// Anon Periodic
		assert_ok!(Scheduler::do_schedule(
			DispatchTime::At(2),
			Some((1000, 3)),
			128,
			root(),
			Preimage::bound(call).unwrap(),
		));
		let call =
			Call::Logger(LoggerCall::log { i: 69, weight: call_weight + Weight::from_ref_time(3) });
		// Anon
		assert_ok!(Scheduler::do_schedule(
			DispatchTime::At(2),
			None,
			127,
			root(),
			Preimage::bound(call).unwrap(),
		));
		// Named Periodic
		let call = Call::Logger(LoggerCall::log {
			i: 2600,
			weight: call_weight + Weight::from_ref_time(4),
		});
		assert_ok!(Scheduler::do_schedule_named(
			[2u8; 32],
			DispatchTime::At(1),
			Some((1000, 3)),
			126,
			root(),
			Preimage::bound(call).unwrap(),
		));

		// Will include the named periodic only
		assert_eq!(
			Scheduler::on_initialize(1),
			TestWeightInfo::service_agendas() +
				TestWeightInfo::service_agenda(1) +
				<TestWeightInfo as MarginalWeightInfo>::service_task(None, true, true) +
				TestWeightInfo::execute_dispatch_unsigned() +
				call_weight + Weight::from_ref_time(4)
		);
		assert_eq!(IncompleteSince::<Test>::get(), None);
		assert_eq!(logger::log(), vec![(root(), 2600u32)]);

		// Will include anon and anon periodic
		assert_eq!(
			Scheduler::on_initialize(2),
			TestWeightInfo::service_agendas() +
				TestWeightInfo::service_agenda(2) +
				<TestWeightInfo as MarginalWeightInfo>::service_task(None, false, true) +
				TestWeightInfo::execute_dispatch_unsigned() +
				call_weight + Weight::from_ref_time(3) +
				<TestWeightInfo as MarginalWeightInfo>::service_task(None, false, false) +
				TestWeightInfo::execute_dispatch_unsigned() +
				call_weight + Weight::from_ref_time(2)
		);
		assert_eq!(IncompleteSince::<Test>::get(), None);
		assert_eq!(logger::log(), vec![(root(), 2600u32), (root(), 69u32), (root(), 42u32)]);

		// Will include named only
		assert_eq!(
			Scheduler::on_initialize(3),
			TestWeightInfo::service_agendas() +
				TestWeightInfo::service_agenda(1) +
				<TestWeightInfo as MarginalWeightInfo>::service_task(None, true, false) +
				TestWeightInfo::execute_dispatch_unsigned() +
				call_weight + Weight::from_ref_time(1)
		);
		assert_eq!(IncompleteSince::<Test>::get(), None);
		assert_eq!(
			logger::log(),
			vec![(root(), 2600u32), (root(), 69u32), (root(), 42u32), (root(), 3u32)]
		);

		// Will contain none
		let actual_weight = Scheduler::on_initialize(4);
		assert_eq!(
			actual_weight,
			TestWeightInfo::service_agendas() + TestWeightInfo::service_agenda(0)
		);
	});
}

#[test]
fn root_calls_works() {
	new_test_ext().execute_with(|| {
		let call =
			Box::new(Call::Logger(LoggerCall::log { i: 69, weight: Weight::from_ref_time(10) }));
		let call2 =
			Box::new(Call::Logger(LoggerCall::log { i: 42, weight: Weight::from_ref_time(10) }));
		assert_ok!(Scheduler::schedule_named(Origin::root(), [1u8; 32], 4, None, 127, call,));
		assert_ok!(Scheduler::schedule(Origin::root(), 4, None, 127, call2));
		run_to_block(3);
		// Scheduled calls are in the agenda.
		assert_eq!(Agenda::<Test>::get(4).len(), 2);
		assert!(logger::log().is_empty());
		assert_ok!(Scheduler::cancel_named(Origin::root(), [1u8; 32]));
		assert_ok!(Scheduler::cancel(Origin::root(), 4, 1));
		// Scheduled calls are made NONE, so should not effect state
		run_to_block(100);
		assert!(logger::log().is_empty());
	});
}

#[test]
fn fails_to_schedule_task_in_the_past() {
	new_test_ext().execute_with(|| {
		run_to_block(3);

		let call1 =
			Box::new(Call::Logger(LoggerCall::log { i: 69, weight: Weight::from_ref_time(10) }));
		let call2 =
			Box::new(Call::Logger(LoggerCall::log { i: 42, weight: Weight::from_ref_time(10) }));
		let call3 =
			Box::new(Call::Logger(LoggerCall::log { i: 42, weight: Weight::from_ref_time(10) }));

		assert_err!(
			Scheduler::schedule_named(Origin::root(), [1u8; 32], 2, None, 127, call1),
			Error::<Test>::TargetBlockNumberInPast,
		);

		assert_err!(
			Scheduler::schedule(Origin::root(), 2, None, 127, call2),
			Error::<Test>::TargetBlockNumberInPast,
		);

		assert_err!(
			Scheduler::schedule(Origin::root(), 3, None, 127, call3),
			Error::<Test>::TargetBlockNumberInPast,
		);
	});
}

#[test]
fn should_use_orign() {
	new_test_ext().execute_with(|| {
		let call =
			Box::new(Call::Logger(LoggerCall::log { i: 69, weight: Weight::from_ref_time(10) }));
		let call2 =
			Box::new(Call::Logger(LoggerCall::log { i: 42, weight: Weight::from_ref_time(10) }));
		assert_ok!(Scheduler::schedule_named(
			system::RawOrigin::Signed(1).into(),
			[1u8; 32],
			4,
			None,
			127,
			call,
		));
		assert_ok!(Scheduler::schedule(system::RawOrigin::Signed(1).into(), 4, None, 127, call2,));
		run_to_block(3);
		// Scheduled calls are in the agenda.
		assert_eq!(Agenda::<Test>::get(4).len(), 2);
		assert!(logger::log().is_empty());
		assert_ok!(Scheduler::cancel_named(system::RawOrigin::Signed(1).into(), [1u8; 32]));
		assert_ok!(Scheduler::cancel(system::RawOrigin::Signed(1).into(), 4, 1));
		// Scheduled calls are made NONE, so should not effect state
		run_to_block(100);
		assert!(logger::log().is_empty());
	});
}

#[test]
fn should_check_origin() {
	new_test_ext().execute_with(|| {
		let call =
			Box::new(Call::Logger(LoggerCall::log { i: 69, weight: Weight::from_ref_time(10) }));
		let call2 =
			Box::new(Call::Logger(LoggerCall::log { i: 42, weight: Weight::from_ref_time(10) }));
		assert_noop!(
			Scheduler::schedule_named(
				system::RawOrigin::Signed(2).into(),
				[1u8; 32],
				4,
				None,
				127,
				call
			),
			BadOrigin
		);
		assert_noop!(
			Scheduler::schedule(system::RawOrigin::Signed(2).into(), 4, None, 127, call2),
			BadOrigin
		);
	});
}

#[test]
fn should_check_origin_for_cancel() {
	new_test_ext().execute_with(|| {
		let call = Box::new(Call::Logger(LoggerCall::log_without_filter {
			i: 69,
			weight: Weight::from_ref_time(10),
		}));
		let call2 = Box::new(Call::Logger(LoggerCall::log_without_filter {
			i: 42,
			weight: Weight::from_ref_time(10),
		}));
		assert_ok!(Scheduler::schedule_named(
			system::RawOrigin::Signed(1).into(),
			[1u8; 32],
			4,
			None,
			127,
			call,
		));
		assert_ok!(Scheduler::schedule(system::RawOrigin::Signed(1).into(), 4, None, 127, call2,));
		run_to_block(3);
		// Scheduled calls are in the agenda.
		assert_eq!(Agenda::<Test>::get(4).len(), 2);
		assert!(logger::log().is_empty());
		assert_noop!(
			Scheduler::cancel_named(system::RawOrigin::Signed(2).into(), [1u8; 32]),
			BadOrigin
		);
		assert_noop!(Scheduler::cancel(system::RawOrigin::Signed(2).into(), 4, 1), BadOrigin);
		assert_noop!(Scheduler::cancel_named(system::RawOrigin::Root.into(), [1u8; 32]), BadOrigin);
		assert_noop!(Scheduler::cancel(system::RawOrigin::Root.into(), 4, 1), BadOrigin);
		run_to_block(5);
		assert_eq!(
			logger::log(),
			vec![
				(system::RawOrigin::Signed(1).into(), 69u32),
				(system::RawOrigin::Signed(1).into(), 42u32)
			]
		);
	});
}

#[test]
fn migration_to_v4_works() {
	new_test_ext().execute_with(|| {
		for i in 0..3u64 {
			let k = i.twox_64_concat();
			let old = vec![
				Some(ScheduledV1 {
					maybe_id: None,
					priority: i as u8 + 10,
					call: Call::Logger(LoggerCall::log {
						i: 96,
						weight: Weight::from_ref_time(100),
					}),
					maybe_periodic: None,
				}),
				None,
				Some(ScheduledV1 {
					maybe_id: Some(b"test".to_vec()),
					priority: 123,
					call: Call::Logger(LoggerCall::log {
						i: 69,
						weight: Weight::from_ref_time(10),
					}),
					maybe_periodic: Some((456u64, 10)),
				}),
			];
			frame_support::migration::put_storage_value(b"Scheduler", b"Agenda", &k, old);
		}

		Scheduler::migrate_v1_to_v4();

		let mut x = Agenda::<Test>::iter().map(|x| (x.0, x.1.into_inner())).collect::<Vec<_>>();
		x.sort_by_key(|x| x.0);
		let expected = vec![
			(
				0,
				vec![
					Some(ScheduledOf::<Test> {
						maybe_id: None,
						priority: 10,
						call: Preimage::bound(Call::Logger(LoggerCall::log {
							i: 96,
							weight: Weight::from_ref_time(100),
						}))
						.unwrap(),
						maybe_periodic: None,
						origin: root(),
						_phantom: PhantomData::<u64>::default(),
					}),
					None,
					Some(ScheduledOf::<Test> {
						maybe_id: Some(blake2_256(&b"test"[..])),
						priority: 123,
						call: Preimage::bound(Call::Logger(LoggerCall::log {
							i: 69,
							weight: Weight::from_ref_time(10),
						}))
						.unwrap(),
						maybe_periodic: Some((456u64, 10)),
						origin: root(),
						_phantom: PhantomData::<u64>::default(),
					}),
				],
			),
			(
				1,
				vec![
					Some(ScheduledOf::<Test> {
						maybe_id: None,
						priority: 11,
						call: Preimage::bound(Call::Logger(LoggerCall::log {
							i: 96,
							weight: Weight::from_ref_time(100),
						}))
						.unwrap(),
						maybe_periodic: None,
						origin: root(),
						_phantom: PhantomData::<u64>::default(),
					}),
					None,
					Some(ScheduledOf::<Test> {
						maybe_id: Some(blake2_256(&b"test"[..])),
						priority: 123,
						call: Preimage::bound(Call::Logger(LoggerCall::log {
							i: 69,
							weight: Weight::from_ref_time(10),
						}))
						.unwrap(),
						maybe_periodic: Some((456u64, 10)),
						origin: root(),
						_phantom: PhantomData::<u64>::default(),
					}),
				],
			),
			(
				2,
				vec![
					Some(ScheduledOf::<Test> {
						maybe_id: None,
						priority: 12,
						call: Preimage::bound(Call::Logger(LoggerCall::log {
							i: 96,
							weight: Weight::from_ref_time(100),
						}))
						.unwrap(),
						maybe_periodic: None,
						origin: root(),
						_phantom: PhantomData::<u64>::default(),
					}),
					None,
					Some(ScheduledOf::<Test> {
						maybe_id: Some(blake2_256(&b"test"[..])),
						priority: 123,
						call: Preimage::bound(Call::Logger(LoggerCall::log {
							i: 69,
							weight: Weight::from_ref_time(10),
						}))
						.unwrap(),
						maybe_periodic: Some((456u64, 10)),
						origin: root(),
						_phantom: PhantomData::<u64>::default(),
					}),
				],
			),
		];
		for (i, j) in x.iter().zip(expected.iter()) {
			assert_eq!(i.0, j.0);
			for (x, y) in i.1.iter().zip(j.1.iter()) {
				assert_eq!(x, y);
			}
		}
		assert_eq_uvec!(x, expected);

		assert_eq!(Scheduler::current_storage_version(), 3);
	});
}

#[test]
fn test_migrate_origin() {
	new_test_ext().execute_with(|| {
		for i in 0..3u64 {
			let k = i.twox_64_concat();
			let old: Vec<Option<Scheduled<Bounded<Call>, u64, u32, u64>>> = vec![
				Some(Scheduled {
					maybe_id: None,
					priority: i as u8 + 10,
					call: Preimage::bound(Call::Logger(LoggerCall::log {
						i: 96,
						weight: Weight::from_ref_time(100),
					}))
					.unwrap(),
					origin: 3u32,
					maybe_periodic: None,
					_phantom: Default::default(),
				}),
				None,
				Some(Scheduled {
					maybe_id: Some(blake2_256(&b"test"[..])),
					priority: 123,
					origin: 2u32,
					call: Preimage::bound(Call::Logger(LoggerCall::log {
						i: 69,
						weight: Weight::from_ref_time(10),
					}))
					.unwrap(),
					maybe_periodic: Some((456u64, 10)),
					_phantom: Default::default(),
				}),
			];
			frame_support::migration::put_storage_value(b"Scheduler", b"Agenda", &k, old);
		}

		impl Into<OriginCaller> for u32 {
			fn into(self) -> OriginCaller {
				match self {
					3u32 => system::RawOrigin::Root.into(),
					2u32 => system::RawOrigin::None.into(),
					_ => unreachable!("test make no use of it"),
				}
			}
		}

		Scheduler::migrate_origin::<u32>();

		assert_eq_uvec!(
			Agenda::<Test>::iter().map(|x| (x.0, x.1.into_inner())).collect::<Vec<_>>(),
			vec![
				(
					0,
					vec![
						Some(ScheduledOf::<Test> {
							maybe_id: None,
							priority: 10,
							call: Preimage::bound(Call::Logger(LoggerCall::log {
								i: 96,
								weight: Weight::from_ref_time(100)
							}))
							.unwrap(),
							maybe_periodic: None,
							origin: system::RawOrigin::Root.into(),
							_phantom: PhantomData::<u64>::default(),
						}),
						None,
						Some(Scheduled {
							maybe_id: Some(blake2_256(&b"test"[..])),
							priority: 123,
							call: Preimage::bound(Call::Logger(LoggerCall::log {
								i: 69,
								weight: Weight::from_ref_time(10)
							}))
							.unwrap(),
							maybe_periodic: Some((456u64, 10)),
							origin: system::RawOrigin::None.into(),
							_phantom: PhantomData::<u64>::default(),
						}),
					]
				),
				(
					1,
					vec![
						Some(Scheduled {
							maybe_id: None,
							priority: 11,
							call: Preimage::bound(Call::Logger(LoggerCall::log {
								i: 96,
								weight: Weight::from_ref_time(100)
							}))
							.unwrap(),
							maybe_periodic: None,
							origin: system::RawOrigin::Root.into(),
							_phantom: PhantomData::<u64>::default(),
						}),
						None,
						Some(Scheduled {
							maybe_id: Some(blake2_256(&b"test"[..])),
							priority: 123,
							call: Preimage::bound(Call::Logger(LoggerCall::log {
								i: 69,
								weight: Weight::from_ref_time(10)
							}))
							.unwrap(),
							maybe_periodic: Some((456u64, 10)),
							origin: system::RawOrigin::None.into(),
							_phantom: PhantomData::<u64>::default(),
						}),
					]
				),
				(
					2,
					vec![
						Some(Scheduled {
							maybe_id: None,
							priority: 12,
							call: Preimage::bound(Call::Logger(LoggerCall::log {
								i: 96,
								weight: Weight::from_ref_time(100)
							}))
							.unwrap(),
							maybe_periodic: None,
							origin: system::RawOrigin::Root.into(),
							_phantom: PhantomData::<u64>::default(),
						}),
						None,
						Some(Scheduled {
							maybe_id: Some(blake2_256(&b"test"[..])),
							priority: 123,
							call: Preimage::bound(Call::Logger(LoggerCall::log {
								i: 69,
								weight: Weight::from_ref_time(10)
							}))
							.unwrap(),
							maybe_periodic: Some((456u64, 10)),
							origin: system::RawOrigin::None.into(),
							_phantom: PhantomData::<u64>::default(),
						}),
					]
				)
			]
		);
	});
}

/// The scheduler postpones calls without preimage forever but does not re-schedule them.
#[test]
fn postponed_named_task_can_be_rescheduled() {
	new_test_ext().execute_with(|| {
		let call = Call::Logger(LoggerCall::log { i: 42, weight: Weight::from_ref_time(1000) });
		let hash = <Test as frame_system::Config>::Hashing::hash_of(&call);
		let len = call.using_encoded(|x| x.len()) as u32;
		let hashed = Preimage::pick(hash, len);
		let name: [u8; 32] = hash.as_ref().try_into().unwrap();

		assert_ok!(Scheduler::do_schedule_named(
			name,
			DispatchTime::At(4),
			None,
			127,
			root(),
			hashed.clone()
		));
		assert!(Preimage::is_requested(&hash));
		assert!(Lookup::<Test>::contains_key(name));

		// Run to a very large block.
		run_to_block(10);
		// It was not executed.
		assert!(logger::log().is_empty());
		assert!(Preimage::is_requested(&hash));
		// TODO Postponing removes the lookup.
		assert!(!Lookup::<Test>::contains_key(name));

		// The agenda still contains the call.
		let agenda = Agenda::<Test>::iter().collect::<Vec<_>>();
		assert_eq!(agenda.len(), 1);
		assert_eq!(
			agenda[0].1,
			vec![Some(Scheduled {
				maybe_id: Some(name),
				priority: 127,
				call: hashed,
				maybe_periodic: None,
				origin: root().into(),
				_phantom: Default::default(),
			})]
		);

		// Finally add the preimage.
		assert_ok!(Preimage::note(call.encode().into()));
		run_to_block(1000);
		// It did not execute.
		assert!(logger::log().is_empty());
		assert!(Preimage::is_requested(&hash));

		// Manually re-schedule the call.
		// TODO this fails since the postpone removed the Lookup.
		assert_ok!(Scheduler::do_reschedule_named(name, DispatchTime::At(1001)));
		run_to_block(1001);

		// It executed:
		assert_eq!(logger::log(), vec![(root(), 42)]);
		assert!(!Preimage::is_requested(&hash));

		let agenda = Agenda::<Test>::iter().collect::<Vec<_>>();
		assert_eq!(agenda.len(), 1);
		assert_eq!(agenda[0].1, vec![None]);
	});
}
