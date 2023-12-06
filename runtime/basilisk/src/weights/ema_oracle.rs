// This file is part of Basilisk.

// Copyright (C) 2020-2023  Intergalactic, Limited (GIB).
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.


//! Autogenerated weights for `pallet_ema_oracle`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-12-06, STEPS: `10`, REPEAT: `30`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `bench-bot`, CPU: `Intel(R) Core(TM) i7-7700K CPU @ 4.20GHz`
//! WASM-EXECUTION: `Compiled`, CHAIN: `Some("dev")`, DB CACHE: `1024`

// Executed Command:
// target/release/basilisk
// benchmark
// pallet
// --chain=dev
// --steps=10
// --repeat=30
// --wasm-execution=compiled
// --heap-pages=4096
// --template=.maintain/pallet-weight-template-no-back.hbs
// --pallet=pallet-ema-oracle
// --output=ema-oracle.rs
// --extrinsic=*

#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(clippy::unnecessary_cast)]

use frame_support::{
	traits::Get,
	weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;

use pallet_ema_oracle::weights::WeightInfo;

pub struct BasiliskWeight<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for BasiliskWeight<T> {
	/// Storage: `EmaOracle::Accumulator` (r:1 w:0)
	/// Proof: `EmaOracle::Accumulator` (`max_values`: Some(1), `max_size`: Some(4441), added: 4936, mode: `MaxEncodedLen`)
	fn on_finalize_no_entry() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `109`
		//  Estimated: `5926`
		// Minimum execution time: 3_828_000 picoseconds.
		Weight::from_parts(3_986_000, 5926)
			.saturating_add(T::DbWeight::get().reads(1_u64))
	}
	/// Storage: `EmaOracle::Accumulator` (r:1 w:1)
	/// Proof: `EmaOracle::Accumulator` (`max_values`: Some(1), `max_size`: Some(4441), added: 4936, mode: `MaxEncodedLen`)
	/// Storage: `EmaOracle::Oracles` (r:145 w:145)
	/// Proof: `EmaOracle::Oracles` (`max_values`: None, `max_size`: Some(177), added: 2652, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 29]`.
	fn on_finalize_multiple_tokens(b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `170 + b * (933 ±0)`
		//  Estimated: `5926 + b * (13260 ±0)`
		// Minimum execution time: 73_522_000 picoseconds.
		Weight::from_parts(12_421_249, 5926)
			// Standard Error: 46_280
			.saturating_add(Weight::from_parts(60_856_147, 0).saturating_mul(b.into()))
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().reads((5_u64).saturating_mul(b.into())))
			.saturating_add(T::DbWeight::get().writes(1_u64))
			.saturating_add(T::DbWeight::get().writes((5_u64).saturating_mul(b.into())))
			.saturating_add(Weight::from_parts(0, 13260).saturating_mul(b.into()))
	}
	/// Storage: `EmaOracle::Accumulator` (r:1 w:1)
	/// Proof: `EmaOracle::Accumulator` (`max_values`: Some(1), `max_size`: Some(4441), added: 4936, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 29]`.
	fn on_trade_multiple_tokens(b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `171 + b * (148 ±0)`
		//  Estimated: `5926`
		// Minimum execution time: 9_281_000 picoseconds.
		Weight::from_parts(9_455_551, 5926)
			// Standard Error: 2_484
			.saturating_add(Weight::from_parts(384_006, 0).saturating_mul(b.into()))
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: `EmaOracle::Accumulator` (r:1 w:1)
	/// Proof: `EmaOracle::Accumulator` (`max_values`: Some(1), `max_size`: Some(4441), added: 4936, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 29]`.
	fn on_liquidity_changed_multiple_tokens(b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `171 + b * (148 ±0)`
		//  Estimated: `5926`
		// Minimum execution time: 9_311_000 picoseconds.
		Weight::from_parts(9_473_229, 5926)
			// Standard Error: 2_600
			.saturating_add(Weight::from_parts(390_209, 0).saturating_mul(b.into()))
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: `EmaOracle::Oracles` (r:2 w:0)
	/// Proof: `EmaOracle::Oracles` (`max_values`: None, `max_size`: Some(177), added: 2652, mode: `MaxEncodedLen`)
	fn get_entry() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `575`
		//  Estimated: `6294`
		// Minimum execution time: 19_841_000 picoseconds.
		Weight::from_parts(20_202_000, 6294)
			.saturating_add(T::DbWeight::get().reads(2_u64))
	}
}
