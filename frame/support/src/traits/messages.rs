// This file is part of Substrate.

// Copyright (C) 2019-2022 Parity Technologies (UK) Ltd.
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

//! Traits for managing message queuing and handling.

use codec::{Decode, Encode, FullCodec, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::{ConstU32, Get, TypedGet};
use sp_runtime::{traits::Convert, BoundedSlice, RuntimeDebug};
use sp_std::{fmt::Debug, marker::PhantomData, prelude::*};
use sp_weights::Weight;

#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, TypeInfo, RuntimeDebug)]
pub enum ProcessMessageError {
	/// The message data format is unknown (e.g. unrecognised header)
	BadFormat,
	/// The message data is bad (e.g. decoding returns an error).
	Corrupt,
	/// The message format is unsupported (e.g. old XCM version).
	Unsupported,
	/// Message processing was not attempted because it was not certain that the weight limit
	/// would be respected. The parameter gives the maximum weight which the message could take
	/// to process.
	Overweight(Weight),
}

pub trait ProcessMessage {
	/// The transport from where a message originates.
	type Origin: FullCodec + MaxEncodedLen + Clone + Eq + PartialEq + TypeInfo + Debug;

	/// Process the given message, using no more than `weight_limit` in weight to do so.
	fn process_message(
		message: &[u8],
		origin: Self::Origin,
		weight_limit: Weight,
	) -> Result<(bool, Weight), ProcessMessageError>;
}

pub trait ServiceQueues {
	/// Service all message queues in some fair manner.
	///
	/// - `weight_limit`: The maximum amount of dynamic weight that this call can use.
	///
	/// Returns the dynamic weight used by this call; is never greater than `weight_limit`.
	fn service_queues(weight_limit: Weight) -> Weight;
}

#[derive(Default, Copy, Clone, Eq, PartialEq, RuntimeDebug)]
pub struct Footprint {
	pub count: u32,
	pub size: u32,
}

pub trait EnqueueMessage<Origin: MaxEncodedLen> {
	type MaxMessageLen: Get<u32>;

	/// Enqueue a single `message` from a specific `origin`.
	fn enqueue_message(message: BoundedSlice<u8, Self::MaxMessageLen>, origin: Origin);

	/// Enqueue multiple `messages` from a specific `origin`.
	fn enqueue_messages<'a>(
		messages: impl Iterator<Item = BoundedSlice<'a, u8, Self::MaxMessageLen>>,
		origin: Origin,
	);

	/// Any remaining unprocessed messages should happen only lazily, not proactively.
	fn sweep_queue(origin: Origin);

	/// Return the state footprint of the given queue.
	fn footprint(origin: Origin) -> Footprint;
}

impl<Origin: MaxEncodedLen> EnqueueMessage<Origin> for () {
	type MaxMessageLen = ConstU32<0>;
	fn enqueue_message(_: BoundedSlice<u8, Self::MaxMessageLen>, _: Origin) {}
	fn enqueue_messages<'a>(
		_: impl Iterator<Item = BoundedSlice<'a, u8, Self::MaxMessageLen>>,
		_: Origin,
	) {
	}
	fn sweep_queue(_: Origin) {}
	fn footprint(_: Origin) -> Footprint {
		Footprint::default()
	}
}

pub struct TransformOrigin<E, O, N, C>(PhantomData<(E, O, N, C)>);
impl<E: EnqueueMessage<O>, O: MaxEncodedLen, N: MaxEncodedLen, C: Convert<N, O>> EnqueueMessage<N>
	for TransformOrigin<E, O, N, C>
{
	type MaxMessageLen = E::MaxMessageLen;

	fn enqueue_message(message: BoundedSlice<u8, Self::MaxMessageLen>, origin: N) {
		E::enqueue_message(message, C::convert(origin));
	}

	fn enqueue_messages<'a>(
		messages: impl Iterator<Item = BoundedSlice<'a, u8, Self::MaxMessageLen>>,
		origin: N,
	) {
		E::enqueue_messages(messages, C::convert(origin));
	}

	fn sweep_queue(origin: N) {
		E::sweep_queue(C::convert(origin));
	}

	fn footprint(origin: N) -> Footprint {
		E::footprint(C::convert(origin))
	}
}

pub trait HandleMessage {
	type MaxMessageLen: Get<u32>;

	/// Enqueue a single `message` with an implied origin.
	fn handle_message(message: BoundedSlice<u8, Self::MaxMessageLen>);

	/// Enqueue multiple `messages` from an implied origin.
	fn handle_messages<'a>(
		messages: impl Iterator<Item = BoundedSlice<'a, u8, Self::MaxMessageLen>>,
	);

	/// Any remaining unprocessed messages should happen only lazily, not proactively.
	fn sweep_queue();

	/// Return the state footprint of the queue.
	fn footprint() -> Footprint;
}

pub struct EnqueueWithOrigin<E, O>(PhantomData<(E, O)>);
impl<E: EnqueueMessage<O::Type>, O: TypedGet> HandleMessage for EnqueueWithOrigin<E, O>
where
	O::Type: MaxEncodedLen,
{
	type MaxMessageLen = E::MaxMessageLen;

	fn handle_message(message: BoundedSlice<u8, Self::MaxMessageLen>) {
		E::enqueue_message(message, O::get());
	}

	fn handle_messages<'a>(
		messages: impl Iterator<Item = BoundedSlice<'a, u8, Self::MaxMessageLen>>,
	) {
		E::enqueue_messages(messages, O::get());
	}

	fn sweep_queue() {
		E::sweep_queue(O::get());
	}

	fn footprint() -> Footprint {
		E::footprint(O::get())
	}
}
