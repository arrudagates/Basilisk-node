// This file is part of HydraDXqq.

// Copyright (C) 2020-2021  Intergalactic, Limited (GIB).
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// Abbr:
//  rps - reward per share
//
//  TODO:
//      * weights and benchmarking
//      * mining nft on deposit_shares()
//      * weighted pools
//      * add real reward curve
//      * add canceled check to all user actions

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

mod benchmarking;
pub mod weights;

pub use pallet::*;

type PoolId = u32;

use frame_support::{
	ensure,
	sp_runtime::traits::{BlockNumberProvider, One, Zero},
	transactional, PalletId,
};

use sp_arithmetic::{
	traits::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub},
	FixedU128, Permill,
};

use orml_traits::{MultiCurrency, MultiCurrencyExtended};

use codec::{Decode, Encode};
use frame_support::sp_runtime::{traits::AccountIdConversion, RuntimeDebug};

impl Default for LoyaltyCurve {
	fn default() -> Self {
		Self {
			b: FixedU128::from_inner(500_000_000_000_000_000), // 0.5
			scale_coef: 100,
		}
	}
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug)]
pub struct GlobalPool<Period, Balance, AssetId, AccountId, BlockNumber: AtLeast32BitUnsigned + Copy> {
	id: PoolId,
	owner: AccountId,
	updated_at: Period,
	total_shares: Balance,
	accumulated_rps: Balance,
	accumulated_rps_start: Balance,
	reward_currency: AssetId,
	accumulated_rewards: Balance,
	paid_accumulated_rewards: Balance,
	yield_per_period: Permill,
	planned_yielding_periods: Period,
	blocks_per_period: BlockNumber,
	incentivized_token: AssetId,
}

impl<Period, Balance: std::default::Default, AssetId, AccountId, BlockNumber: AtLeast32BitUnsigned + Copy>
	GlobalPool<Period, Balance, AssetId, AccountId, BlockNumber>
{
	fn new(
		id: PoolId,
		updated_at: Period,
		reward_currency: AssetId,
		yield_per_period: Permill,
		planned_yielding_periods: Period,
		blocks_per_period: BlockNumber,
		owner: AccountId,
		incentivized_token: AssetId,
	) -> Self {
		Self {
			accumulated_rewards: Default::default(),
			accumulated_rps: Default::default(),
			accumulated_rps_start: Default::default(),
			paid_accumulated_rewards: Default::default(),
			total_shares: Default::default(),
			id,
			updated_at,
			reward_currency,
			yield_per_period,
			planned_yielding_periods,
			blocks_per_period,
			owner,
			incentivized_token,
		}
	}
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug)]
pub struct Pool<Period, Balance> {
	updated_at: Period,
	total_shares: Balance,
	accumulated_rps: Balance,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug)]
pub struct LoyaltyCurve {
	b: FixedU128,
	scale_coef: u32,
}

use frame_support::pallet_prelude::*;
use frame_support::sp_runtime::{traits::AtLeast32BitUnsigned, FixedPointNumber, FixedPointOperand};
use sp_std::convert::{From, Into, TryInto};

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use crate::weights::WeightInfo;
	use frame_system::pallet_prelude::{BlockNumberFor, OriginFor};

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Balance type
		type Balance: Parameter
			+ From<u128>
			+ Into<u128>
			+ From<Self::BlockNumber>
			//+ From<FixedU128>
			+ Member
			+ AtLeast32BitUnsigned
			+ FixedPointOperand
			+ Default
			+ Copy
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen
			+ CheckedAdd;

		/// Asset type
		type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize + Ord + From<u32>;

		/// Currency for transfers
		type MultiCurrency: MultiCurrencyExtended<
			Self::AccountId,
			CurrencyId = Self::CurrencyId,
			Balance = Self::Balance,
		>;

		/// The origin account that cat create new liquidity mining program
		type CreateOrigin: EnsureOrigin<Self::Origin>;

		type PalletId: Get<PalletId>;

		/// Minimum anout of total rewards to create new farm
		type MinTotalFarmRewards: Get<Self::Balance>;

		/// Minimalnumber of periods to distribute farm rewards
		type MinPlannedYieldingPeriods: Get<Self::BlockNumber>;

		/// The block number provider
		type BlockNumberProvider: BlockNumberProvider<BlockNumber = Self::BlockNumber>;

		/// Weight information for extrinsics in this module.
		type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	#[derive(PartialEq)]
	pub enum Error<T> {
		/// Math computation overflow
		Overflow,

		/// Insufficient balance in global pool to transfer rewards to pool
		InsufficientBalancdInGlobalPool,

		/// Provide id is not valid. Valid range is [1, u32::MAX)
		InvalidPoolId,

		/// Provided planed_yielding_periods is below limit
		InvalidPlannedYieldingPeriods,

		/// Provided blocks_per_period can't be 0
		InvalidBlocksPerPeriod,

		/// Yield per period can't be 0
		InvalidYieldPerPeriod,

		/// Provided total_rewards for farming is bellow min limit
		InvalidTotalRewards,

		/// Reward currency balance too low
		InsufficientRewardCurrencyBalance,

		/// Feature is not implemented yet
		NotImplemented,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New farm was creaated by `CreateOrigin` origin. [pool_id, global_pool]
		FarmCreated(
			PoolId,
			GlobalPool<T::BlockNumber, T::Balance, T::CurrencyId, T::AccountId, T::BlockNumber>,
		),
	}

	#[pallet::storage]
	#[pallet::getter(fn pool_id)]
	pub type PoolIdSeq<T: Config> = StorageValue<_, PoolId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn global_pool)]
	pub type GlobalPoolData<T: Config> = StorageMap<
		_,
		Twox64Concat,
		PoolId,
		GlobalPool<T::BlockNumber, T::Balance, T::CurrencyId, T::AccountId, T::BlockNumber>,
		OptionQuery,
	>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create new liquidity mining program
		///
		/// Parameters:
		/// - `origin`:
		/// - `total_rewards`:
		/// - `planned_yielding_periods`: planned number of periods to distribute rewards. WARN: this is not
		/// how long will farming run.  Owner can destroy farm sooner or liq. mining can run longer
		/// if all the rewards will not distributed.
		/// - `blocks_per_period`:
		/// - `incetivized_token`
		/// - `reward_currency`
		/// - `admin_account`
		/// - `yield_per_period`
		#[pallet::weight(1000)]
		#[transactional]
		pub fn create_farm(
			origin: OriginFor<T>,
			total_rewards: T::Balance,
			planned_yielding_periods: T::BlockNumber,
			blocks_per_period: T::BlockNumber,
			incentivized_token: T::CurrencyId,
			reward_currency: T::CurrencyId,
			owner: T::AccountId,
			yield_per_period: Permill,
		) -> DispatchResult {
			T::CreateOrigin::ensure_origin(origin)?;

			Self::validate_create_new_farm_data(
				total_rewards,
				planned_yielding_periods,
				blocks_per_period,
				yield_per_period,
			)?;

			ensure!(
				T::MultiCurrency::free_balance(reward_currency, &owner) >= total_rewards,
				Error::<T>::InsufficientRewardCurrencyBalance
			);

			let now_period =
				Self::get_period_number(T::BlockNumberProvider::current_block_number(), blocks_per_period)?;
			let pool_id = Self::get_next_id()?;

			let pool = GlobalPool::new(
				pool_id,
				now_period,
				reward_currency,
				yield_per_period,
				planned_yielding_periods,
				blocks_per_period,
				owner,
				incentivized_token,
			);

			<GlobalPoolData<T>>::insert(&pool.id, &pool);

			let pool_account = Self::pool_account_id(pool.id)?;
			T::MultiCurrency::transfer(reward_currency, &pool.owner, &pool_account, total_rewards)?;

			Self::deposit_event(Event::FarmCreated(pool.id, pool));

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn get_next_id() -> Result<PoolId, Error<T>> {
		let next_id = PoolIdSeq::<T>::try_mutate(|current_id| {
			*current_id = current_id.checked_add(1).ok_or(Error::<T>::Overflow)?;

			Ok(*current_id)
		})?;

		Ok(next_id)
	}

	/// Account id of pot holding all the shares
	pub fn account_id() {
		T::PalletId::get().into_account()
	}

	/// Return pallet account or pool acocunt from PoolId
	///
	/// WARN: pool_id = 0 is same as `T::PalletId::get().into_account()`. 0 is not valid value
	pub fn pool_account_id(pool_id: PoolId) -> Result<T::AccountId, Error<T>> {
		Self::validate_pool_id(pool_id)?;

		Ok(T::PalletId::get().into_sub_account(pool_id))
	}

	/// Return period number from block number now and number of blocks in one period
	pub fn get_period_number(
		now: T::BlockNumber,
		blocks_per_period: T::BlockNumber,
	) -> Result<T::BlockNumber, Error<T>> {
		if blocks_per_period.is_one() {
			return Ok(now);
		}

		now.checked_div(&blocks_per_period).ok_or(Error::<T>::Overflow)
	}

	/// Loyalty multiplier  
	///
	// theta = periods/[(b + 1) * scale_coef];
	//
	// loyalty-multiplier = [theta + (theta * b) + b]/[theta + (theta * b) + 1]
	//
	pub fn get_loyalty_multiplier(periods: T::BlockNumber, curve: &LoyaltyCurve) -> Result<FixedU128, Error<T>> {
		//b.is_one() is special case
		if FixedPointNumber::is_one(&curve.b) {
			return Ok(1.into());
		}

		let denom = curve
			.b
			.checked_add(&1.into())
			.ok_or(Error::<T>::Overflow)?
			.checked_mul(&FixedU128::from(Into::<u128>::into(curve.scale_coef)))
			.ok_or(Error::<T>::Overflow)?;

		let p = FixedU128::from(TryInto::<u128>::try_into(periods).map_err(|_e| Error::<T>::Overflow)?);
		let theta = p.checked_div(&denom).ok_or(Error::<T>::Overflow)?;

		let theta_mul_b = theta.checked_mul(&curve.b).ok_or(Error::<T>::Overflow)?;

		let theta_add_theta_mul_b = theta.checked_add(&theta_mul_b).ok_or(Error::<T>::Overflow)?;

		let num = theta_add_theta_mul_b
			.checked_add(&curve.b)
			.ok_or(Error::<T>::Overflow)?;

		let denom = theta_add_theta_mul_b
			.checked_add(&1.into())
			.ok_or(Error::<T>::Overflow)?;

		num.checked_div(&denom).ok_or(Error::<T>::Overflow)
	}

	pub fn get_reward_per_period(
		yield_per_period: FixedU128,
		total_global_farm_shares: T::Balance,
		max_reward_per_period: T::Balance,
	) -> Result<T::Balance, Error<T>> {
		Ok(yield_per_period
			.checked_mul_int(total_global_farm_shares)
			.ok_or(Error::<T>::Overflow)?
			.min(max_reward_per_period))
	}

	pub fn update_global_pool(
		pool_id: PoolId,
		pool: &mut GlobalPool<T::BlockNumber, T::Balance, T::CurrencyId, T::AccountId, T::BlockNumber>,
		now_period: T::BlockNumber,
		reward_per_period: T::Balance,
	) -> Result<(), Error<T>> {
		if pool.updated_at == now_period {
			return Ok(());
		}

		if pool.total_shares.is_zero() {
			return Ok(());
		}

		let periods_since_last_update: T::Balance =
			TryInto::<u128>::try_into(now_period.checked_sub(&pool.updated_at).ok_or(Error::<T>::Overflow)?)
				.map_err(|_e| Error::<T>::Overflow)?
				.into();

		let pool_account = Self::pool_account_id(pool_id);
		let reward = periods_since_last_update
			.checked_mul(&reward_per_period)
			.ok_or(Error::<T>::Overflow)?
			.min(T::MultiCurrency::free_balance(pool.reward_currency, &pool_account?));

		if !reward.is_zero() {
			pool.accumulated_rps = Self::get_accumulated_rps(pool.accumulated_rps, pool.total_shares, reward)?;

			pool.accumulated_rewards = pool
				.accumulated_rewards
				.checked_add(&reward)
				.ok_or(Error::<T>::Overflow)?;
		}

		pool.updated_at = now_period;

		return Ok(());
	}

	pub fn get_accumulated_rps(
		accumulated_rps_now: T::Balance,
		total_shares: T::Balance,
		reward: T::Balance,
	) -> Result<T::Balance, Error<T>> {
		reward
			.checked_div(&total_shares)
			.ok_or(Error::<T>::Overflow)?
			.checked_add(&accumulated_rps_now)
			.ok_or(Error::<T>::Overflow)
	}

	/// (user_rewards, unclaimable_rewards)
	/// NOTE: claimable_reward and user_rewards is not the same !!!
	pub fn get_user_reward(
		user_accumulated_rps: T::Balance,
		user_shares: T::Balance,
		accumulated_rps_now: T::Balance,
		user_accumulated_claimed_rewards: T::Balance,
		loyalty_multiplier: FixedU128,
	) -> Result<(T::Balance, T::Balance), Error<T>> {
		let max_rewards = accumulated_rps_now
			.checked_sub(&user_accumulated_rps)
			.ok_or(Error::<T>::Overflow)?
			.checked_mul(&user_shares)
			.ok_or(Error::<T>::Overflow)?;

		let claimable_rewards = loyalty_multiplier
			.checked_mul_int(max_rewards)
			.ok_or(Error::<T>::Overflow)?;

		let unclaimable_rewards = max_rewards
			.checked_sub(&claimable_rewards)
			.ok_or(Error::<T>::Overflow)?;

		let user_rewards = claimable_rewards
			.checked_sub(&user_accumulated_claimed_rewards)
			.ok_or(Error::<T>::Overflow)?;

		Ok((user_rewards, unclaimable_rewards))
	}

	pub fn claim_from_global_pool(
		pool: &mut GlobalPool<T::BlockNumber, T::Balance, T::CurrencyId, T::AccountId, T::BlockNumber>,
		shares: T::Balance,
	) -> Result<T::Balance, Error<T>> {
		let reward = pool
			.accumulated_rps
			.checked_sub(&pool.accumulated_rps_start)
			.ok_or(Error::<T>::Overflow)?
			.checked_mul(&shares)
			.ok_or(Error::<T>::Overflow)?
			.min(pool.accumulated_rewards);

		pool.accumulated_rps_start = pool.accumulated_rps;

		pool.paid_accumulated_rewards = pool
			.paid_accumulated_rewards
			.checked_add(&reward)
			.ok_or(Error::<T>::Overflow)?;

		pool.accumulated_rewards = pool
			.accumulated_rewards
			.checked_sub(&reward)
			.ok_or(Error::<T>::Overflow)?;

		return Ok(reward);
	}

	pub fn update_pool(
		pool_id: PoolId,
		pool: &mut Pool<T::BlockNumber, T::Balance>,
		rewards: T::Balance,
		period_now: T::BlockNumber,
		global_pool_id: PoolId,
		reward_currency: T::CurrencyId,
	) -> Result<(), DispatchError> {
		if pool.updated_at == period_now {
			return Ok(());
		}

		if pool.total_shares.is_zero() {
			return Ok(());
		}

		pool.accumulated_rps = Self::get_accumulated_rps(pool.accumulated_rps, pool.total_shares, rewards)?;
		pool.updated_at = period_now;

		let global_pool_balance =
			T::MultiCurrency::free_balance(reward_currency, &Self::pool_account_id(global_pool_id)?);

		ensure!(
			global_pool_balance >= rewards,
			Error::<T>::InsufficientBalancdInGlobalPool
		);

		let global_pool_account = Self::pool_account_id(global_pool_id)?;
		let pool_account = Self::pool_account_id(pool_id)?;
		T::MultiCurrency::transfer(reward_currency, &global_pool_account, &pool_account, rewards)
	}

	pub fn validate_pool_id(pool_id: PoolId) -> Result<(), Error<T>> {
		if pool_id.le(&Zero::zero()) {
			return Err(Error::<T>::InvalidPoolId);
		}

		Ok(())
	}

	pub fn validate_create_new_farm_data(
		total_rewads: T::Balance,
		planned_yielding_periods: T::BlockNumber,
		blocks_per_period: T::BlockNumber,
		yield_per_period: Permill,
	) -> DispatchResult {
		ensure!(
			total_rewads >= T::MinTotalFarmRewards::get(),
			Error::<T>::InvalidTotalRewards
		);

		ensure!(
			planned_yielding_periods >= T::MinPlannedYieldingPeriods::get(),
			Error::<T>::InvalidPlannedYieldingPeriods
		);

		ensure!(!Zero::is_zero(&blocks_per_period), Error::<T>::InvalidBlocksPerPeriod);

		ensure!(!yield_per_period.is_zero(), Error::<T>::InvalidYieldPerPeriod);

		Ok(())
	}
}
