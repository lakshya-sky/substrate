
//! Autogenerated weights for pallet_message_queue
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-10-28, STEPS: `2`, REPEAT: 1, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `oty-parity`, CPU: `11th Gen Intel(R) Core(TM) i7-1165G7 @ 2.80GHz`
//! EXECUTION: None, WASM-EXECUTION: Compiled, CHAIN: None, DB CACHE: 1024

// Executed Command:
// ./target/release/substrate
// benchmark
// pallet
// --dev
// --pallet
// pallet-message-queue
// --extrinsic=
// --output
// frame/message-queue/src/weights.rs
// --template
// .maintain/frame-weight-template.hbs
// --steps
// 2
// --repeat
// 1

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;
use sp_std::vec::Vec;

/// Weight functions needed for pallet_message_queue.
pub trait WeightInfo {
	fn service_page_base() -> Weight;
	fn service_queue_base() -> Weight;
	fn service_page_process_message() -> Weight;
	fn bump_service_head() -> Weight;
	fn service_page_item() -> Weight;
}

// TODO auto-generate this by the benchmarking
pub struct WeightMetaInfo<W>(PhantomData<W>);
impl<W: WeightInfo> WeightMetaInfo<W> {
	/// Executes a callback for each weight function by passing in its name and value.
	pub fn visit_weight_functions<F: Fn(&'static str, Weight) -> R, R>(f: F) -> Vec<R> {
		sp_std::vec![
			f("service_page_base", W::service_page_base()),
			f("service_queue_base", W::service_queue_base()),
			f("service_page_process_message", W::service_page_process_message()),
			f("bump_service_head", W::bump_service_head()),
			f("service_page_item", W::service_page_item()),
		]
	}
}

/// Weights for pallet_message_queue using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	fn service_page_base() -> Weight {
		// Minimum execution time: 86 nanoseconds.
		Weight::from_ref_time(86_000 as u64)
	}
	fn service_queue_base() -> Weight {
		// Minimum execution time: 112 nanoseconds.
		Weight::from_ref_time(112_000 as u64)
	}

	fn service_page_process_message() -> Weight {
		Weight::from_ref_time(112_000 as u64)
	}

	fn bump_service_head() -> Weight {
		Weight::from_ref_time(112_000 as u64)
	}

	fn service_page_item() -> Weight {
		Weight::from_ref_time(112_000 as u64)
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	fn service_page_base() -> Weight {
		// Minimum execution time: 86 nanoseconds.
		Weight::from_ref_time(86_000 as u64)
	}
	fn service_queue_base() -> Weight {
		// Minimum execution time: 112 nanoseconds.
		Weight::from_ref_time(112_000 as u64)
	}

	fn service_page_process_message() -> Weight {
		Weight::from_ref_time(112_000 as u64)
	}

	fn bump_service_head() -> Weight {
		Weight::from_ref_time(112_000 as u64)
	}

	fn service_page_item() -> Weight {
		Weight::from_ref_time(112_000 as u64)
	}
}