// This file is part of Substrate.

// Copyright (C) 2019-2021 Parity Technologies (UK) Ltd.
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

//! Tests for the alliance pallet.

use frame_support::{assert_noop, assert_ok, error::BadOrigin};
use frame_system::{EventRecord, Phase};

use super::*;
use crate::mock::*;

type AllianceMotionEvent = pallet_collective::Event<Test, pallet_collective::Instance1>;

#[test]
fn force_set_members_works() {
	new_test_ext().execute_with(|| {
		// ensure alliance is set
		assert_eq!(Alliance::votable_members_sorted(), vec![1, 2, 3]);

		// creating and proposing proposals
		let (proposal, proposal_len, hash) = make_remark_proposal(42);
		assert_ok!(Alliance::propose(Origin::signed(1), 3, Box::new(proposal), proposal_len));

		let (k_proposal, k_proposal_len, k_hash) = make_kick_member_proposal(2);
		assert_ok!(Alliance::propose(Origin::signed(1), 3, Box::new(k_proposal), k_proposal_len));

		assert!(Alliance::up_for_kicking(&2));
		// ensure proposal is listed as active proposal
		assert_eq!(<Test as Config>::ProposalProvider::proposals(), vec![hash, k_hash]);

		// fails without root
		assert_noop!(
			Alliance::force_set_members(Origin::signed(1), vec![], vec![], vec![]),
			BadOrigin
		);

		// success call
		assert_ok!(Alliance::force_set_members(Origin::root(), vec![8, 5], vec![4], vec![2]));

		// assert new set of voting members
		assert_eq!(Alliance::votable_members_sorted(), vec![4, 5, 8]);
		// assert new members
		assert!(Alliance::is_ally(&2));
		assert!(!Alliance::up_for_kicking(&2));
		assert!(!Alliance::is_member(&1));
		assert!(!Alliance::is_member(&3));
		// all proposals are removed
		assert_eq!(<Test as Config>::ProposalProvider::proposals(), vec![]);

		System::assert_last_event(mock::Event::Alliance(crate::Event::MembersInitialized {
			founders: vec![5, 8],
			fellows: vec![4],
			allies: vec![2],
		}));
	})
}

#[test]
fn propose_works() {
	new_test_ext().execute_with(|| {
		let (proposal, proposal_len, hash) = make_remark_proposal(42);

		// only votable member can propose proposal, 4 is ally not have vote rights
		assert_noop!(
			Alliance::propose(Origin::signed(4), 3, Box::new(proposal.clone()), proposal_len),
			Error::<Test, ()>::NoVotingRights
		);

		assert_ok!(Alliance::propose(
			Origin::signed(1),
			3,
			Box::new(proposal.clone()),
			proposal_len
		));
		assert_eq!(*AllianceMotion::proposals(), vec![hash]);
		assert_eq!(AllianceMotion::proposal_of(&hash), Some(proposal));
		assert_eq!(
			System::events(),
			vec![EventRecord {
				phase: Phase::Initialization,
				event: mock::Event::AllianceMotion(AllianceMotionEvent::Proposed {
					account: 1,
					proposal_index: 0,
					proposal_hash: hash,
					threshold: 3,
				}),
				topics: vec![],
			}]
		);
	});
}

#[test]
fn vote_works() {
	new_test_ext().execute_with(|| {
		let (proposal, proposal_len, hash) = make_remark_proposal(42);
		assert_ok!(Alliance::propose(
			Origin::signed(1),
			3,
			Box::new(proposal.clone()),
			proposal_len
		));
		assert_ok!(Alliance::vote(Origin::signed(2), hash.clone(), 0, true));

		let record = |event| EventRecord { phase: Phase::Initialization, event, topics: vec![] };
		assert_eq!(
			System::events(),
			vec![
				record(mock::Event::AllianceMotion(AllianceMotionEvent::Proposed {
					account: 1,
					proposal_index: 0,
					proposal_hash: hash.clone(),
					threshold: 3
				})),
				record(mock::Event::AllianceMotion(AllianceMotionEvent::Voted {
					account: 2,
					proposal_hash: hash.clone(),
					voted: true,
					yes: 1,
					no: 0,
				})),
			]
		);
	});
}

#[test]
fn veto_works() {
	new_test_ext().execute_with(|| {
		let (proposal, proposal_len, hash) = make_remark_proposal(42);
		assert_ok!(Alliance::propose(
			Origin::signed(1),
			3,
			Box::new(proposal.clone()),
			proposal_len
		));
		// only set_rule/elevate_ally can be veto
		assert_noop!(
			Alliance::veto(Origin::signed(1), hash.clone()),
			Error::<Test, ()>::NotVetoableProposal
		);

		let cid = test_cid();
		let (vetoable_proposal, vetoable_proposal_len, vetoable_hash) = make_set_rule_proposal(cid);
		assert_ok!(Alliance::propose(
			Origin::signed(1),
			3,
			Box::new(vetoable_proposal.clone()),
			vetoable_proposal_len
		));

		// only founder have veto rights, 3 is fellow
		assert_noop!(
			Alliance::veto(Origin::signed(3), vetoable_hash.clone()),
			Error::<Test, ()>::NotFounder
		);

		assert_ok!(Alliance::veto(Origin::signed(2), vetoable_hash.clone()));
		let record = |event| EventRecord { phase: Phase::Initialization, event, topics: vec![] };
		assert_eq!(
			System::events(),
			vec![
				record(mock::Event::AllianceMotion(AllianceMotionEvent::Proposed {
					account: 1,
					proposal_index: 0,
					proposal_hash: hash.clone(),
					threshold: 3
				})),
				record(mock::Event::AllianceMotion(AllianceMotionEvent::Proposed {
					account: 1,
					proposal_index: 1,
					proposal_hash: vetoable_hash.clone(),
					threshold: 3
				})),
				record(mock::Event::AllianceMotion(AllianceMotionEvent::Disapproved {
					proposal_hash: vetoable_hash.clone()
				})),
			]
		);
	})
}

#[test]
fn close_works() {
	new_test_ext().execute_with(|| {
		let (proposal, proposal_len, hash) = make_remark_proposal(42);
		let proposal_weight = proposal.get_dispatch_info().weight;
		assert_ok!(Alliance::propose(
			Origin::signed(1),
			3,
			Box::new(proposal.clone()),
			proposal_len
		));
		assert_ok!(Alliance::vote(Origin::signed(1), hash.clone(), 0, true));
		assert_ok!(Alliance::vote(Origin::signed(2), hash.clone(), 0, true));
		assert_ok!(Alliance::vote(Origin::signed(3), hash.clone(), 0, true));
		assert_ok!(Alliance::close(
			Origin::signed(1),
			hash.clone(),
			0,
			proposal_weight,
			proposal_len
		));

		let record = |event| EventRecord { phase: Phase::Initialization, event, topics: vec![] };
		assert_eq!(
			System::events(),
			vec![
				record(mock::Event::AllianceMotion(AllianceMotionEvent::Proposed {
					account: 1,
					proposal_index: 0,
					proposal_hash: hash.clone(),
					threshold: 3
				})),
				record(mock::Event::AllianceMotion(AllianceMotionEvent::Voted {
					account: 1,
					proposal_hash: hash.clone(),
					voted: true,
					yes: 1,
					no: 0,
				})),
				record(mock::Event::AllianceMotion(AllianceMotionEvent::Voted {
					account: 2,
					proposal_hash: hash.clone(),
					voted: true,
					yes: 2,
					no: 0,
				})),
				record(mock::Event::AllianceMotion(AllianceMotionEvent::Voted {
					account: 3,
					proposal_hash: hash.clone(),
					voted: true,
					yes: 3,
					no: 0,
				})),
				record(mock::Event::AllianceMotion(AllianceMotionEvent::Closed {
					proposal_hash: hash.clone(),
					yes: 3,
					no: 0,
				})),
				record(mock::Event::AllianceMotion(AllianceMotionEvent::Approved {
					proposal_hash: hash.clone()
				})),
				record(mock::Event::AllianceMotion(AllianceMotionEvent::Executed {
					proposal_hash: hash.clone(),
					result: Err(DispatchError::BadOrigin),
				}))
			]
		);
	});
}

#[test]
fn set_rule_works() {
	new_test_ext().execute_with(|| {
		let cid = test_cid();
		assert_ok!(Alliance::set_rule(Origin::signed(1), cid.clone()));
		assert_eq!(Alliance::rule(), Some(cid.clone()));

		System::assert_last_event(mock::Event::Alliance(crate::Event::NewRuleSet { rule: cid }));
	});
}

#[test]
fn announce_works() {
	new_test_ext().execute_with(|| {
		let cid = test_cid();
		assert_ok!(Alliance::announce(Origin::signed(3), cid.clone()));
		assert_eq!(Alliance::announcements(), vec![cid.clone()]);

		System::assert_last_event(mock::Event::Alliance(crate::Event::Announced {
			announcement: cid,
		}));
	});
}

#[test]
fn remove_announcement_works() {
	new_test_ext().execute_with(|| {
		let cid = test_cid();
		assert_ok!(Alliance::announce(Origin::signed(3), cid.clone()));
		assert_eq!(Alliance::announcements(), vec![cid.clone()]);
		System::assert_last_event(mock::Event::Alliance(crate::Event::Announced {
			announcement: cid.clone(),
		}));

		System::set_block_number(2);

		assert_ok!(Alliance::remove_announcement(Origin::signed(3), cid.clone()));
		assert_eq!(Alliance::announcements(), vec![]);
		System::assert_last_event(mock::Event::Alliance(crate::Event::AnnouncementRemoved {
			announcement: cid,
		}));
	});
}

#[test]
fn join_alliance_works() {
	new_test_ext().execute_with(|| {
		// check already member
		assert_noop!(Alliance::join_alliance(Origin::signed(1)), Error::<Test, ()>::AlreadyMember);

		// check already listed as unscrupulous
		assert_ok!(Alliance::add_unscrupulous_items(
			Origin::signed(3),
			vec![UnscrupulousItem::AccountId(4)]
		));
		assert_noop!(
			Alliance::join_alliance(Origin::signed(4)),
			Error::<Test, ()>::AccountNonGrata
		);
		assert_ok!(Alliance::remove_unscrupulous_items(
			Origin::signed(3),
			vec![UnscrupulousItem::AccountId(4)]
		));

		// check deposit funds
		assert_noop!(
			Alliance::join_alliance(Origin::signed(5)),
			Error::<Test, ()>::InsufficientFunds
		);

		// success to submit
		assert_ok!(Alliance::join_alliance(Origin::signed(4)));
		assert_eq!(Alliance::deposit_of(4), Some(25));
		assert_eq!(Alliance::members(MemberRole::Ally), vec![4]);

		// check already member
		assert_noop!(Alliance::join_alliance(Origin::signed(4)), Error::<Test, ()>::AlreadyMember);

		// check missing identity judgement
		#[cfg(not(feature = "runtime-benchmarks"))]
		assert_noop!(
			Alliance::join_alliance(Origin::signed(6)),
			Error::<Test, ()>::WithoutGoodIdentityJudgement
		);
		// check missing identity info
		#[cfg(not(feature = "runtime-benchmarks"))]
		assert_noop!(
			Alliance::join_alliance(Origin::signed(7)),
			Error::<Test, ()>::WithoutIdentityDisplayAndWebsite
		);
	});
}

#[test]
fn nominate_ally_works() {
	new_test_ext().execute_with(|| {
		// check already member
		assert_noop!(
			Alliance::nominate_ally(Origin::signed(1), 2),
			Error::<Test, ()>::AlreadyMember
		);

		// only votable member(founder/fellow) have nominate right
		assert_noop!(
			Alliance::nominate_ally(Origin::signed(5), 4),
			Error::<Test, ()>::NoVotingRights
		);

		// check already listed as unscrupulous
		assert_ok!(Alliance::add_unscrupulous_items(
			Origin::signed(3),
			vec![UnscrupulousItem::AccountId(4)]
		));
		assert_noop!(
			Alliance::nominate_ally(Origin::signed(1), 4),
			Error::<Test, ()>::AccountNonGrata
		);
		assert_ok!(Alliance::remove_unscrupulous_items(
			Origin::signed(3),
			vec![UnscrupulousItem::AccountId(4)]
		));

		// success to nominate
		assert_ok!(Alliance::nominate_ally(Origin::signed(1), 4));
		assert_eq!(Alliance::deposit_of(4), None);
		assert_eq!(Alliance::members(MemberRole::Ally), vec![4]);

		// check already member
		assert_noop!(
			Alliance::nominate_ally(Origin::signed(1), 4),
			Error::<Test, ()>::AlreadyMember
		);

		// check missing identity judgement
		#[cfg(not(feature = "runtime-benchmarks"))]
		assert_noop!(
			Alliance::join_alliance(Origin::signed(6)),
			Error::<Test, ()>::WithoutGoodIdentityJudgement
		);
		// check missing identity info
		#[cfg(not(feature = "runtime-benchmarks"))]
		assert_noop!(
			Alliance::join_alliance(Origin::signed(7)),
			Error::<Test, ()>::WithoutIdentityDisplayAndWebsite
		);
	});
}

#[test]
fn elevate_ally_works() {
	new_test_ext().execute_with(|| {
		assert_noop!(Alliance::elevate_ally(Origin::signed(2), 4), Error::<Test, ()>::NotAlly);

		assert_ok!(Alliance::join_alliance(Origin::signed(4)));
		assert_eq!(Alliance::members(MemberRole::Ally), vec![4]);
		assert_eq!(Alliance::members(MemberRole::Fellow), vec![3]);

		assert_ok!(Alliance::elevate_ally(Origin::signed(2), 4));
		assert_eq!(Alliance::members(MemberRole::Ally), Vec::<u64>::new());
		assert_eq!(Alliance::members(MemberRole::Fellow), vec![3, 4]);
	});
}

#[test]
fn retire_works() {
	new_test_ext().execute_with(|| {
		let (proposal, proposal_len, _) = make_kick_member_proposal(2);
		assert_ok!(Alliance::propose(
			Origin::signed(1),
			3,
			Box::new(proposal.clone()),
			proposal_len
		));
		assert_noop!(Alliance::retire(Origin::signed(2)), Error::<Test, ()>::UpForKicking);

		assert_noop!(Alliance::retire(Origin::signed(4)), Error::<Test, ()>::NotMember);

		assert_eq!(Alliance::members(MemberRole::Fellow), vec![3]);
		assert_ok!(Alliance::retire(Origin::signed(3)));
		assert_eq!(Alliance::members(MemberRole::Fellow), Vec::<u64>::new());
	});
}

#[test]
fn kick_member_works() {
	new_test_ext().execute_with(|| {
		let (proposal, proposal_len, _) = make_kick_member_proposal(2);
		assert_ok!(Alliance::propose(
			Origin::signed(1),
			3,
			Box::new(proposal.clone()),
			proposal_len
		));
		assert_eq!(Alliance::up_for_kicking(2), true);
		assert_eq!(Alliance::members(MemberRole::Founder), vec![1, 2]);

		assert_ok!(Alliance::kick_member(Origin::signed(2), 2));
		assert_eq!(Alliance::members(MemberRole::Founder), vec![1]);
	});
}

#[test]
fn add_unscrupulous_items_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(Alliance::add_unscrupulous_items(
			Origin::signed(3),
			vec![
				UnscrupulousItem::AccountId(3),
				UnscrupulousItem::Website("abc".as_bytes().to_vec().try_into().unwrap())
			]
		));
		assert_eq!(Alliance::unscrupulous_accounts().into_inner(), vec![3]);
		assert_eq!(Alliance::unscrupulous_websites().into_inner(), vec!["abc".as_bytes().to_vec()]);

		assert_noop!(
			Alliance::add_unscrupulous_items(
				Origin::signed(3),
				vec![UnscrupulousItem::AccountId(3)]
			),
			Error::<Test, ()>::AlreadyUnscrupulous
		);
	});
}

#[test]
fn remove_unscrupulous_items_works() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Alliance::remove_unscrupulous_items(
				Origin::signed(3),
				vec![UnscrupulousItem::AccountId(3)]
			),
			Error::<Test, ()>::NotListedAsUnscrupulous
		);

		assert_ok!(Alliance::add_unscrupulous_items(
			Origin::signed(3),
			vec![UnscrupulousItem::AccountId(3)]
		));
		assert_eq!(Alliance::unscrupulous_accounts(), vec![3]);
		assert_ok!(Alliance::remove_unscrupulous_items(
			Origin::signed(3),
			vec![UnscrupulousItem::AccountId(3)]
		));
		assert_eq!(Alliance::unscrupulous_accounts(), Vec::<u64>::new());
	});
}
