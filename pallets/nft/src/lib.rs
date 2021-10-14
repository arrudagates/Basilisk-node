#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::upper_case_acronyms)]

use frame_support::{
	dispatch::DispatchResult,
	traits::{tokens::nonfungibles::*, Currency, NamedReservableCurrency},
	transactional, BoundedVec,
};
use frame_system::ensure_signed;

use primitives::ReserveIdentifier;
use sp_runtime::traits::{CheckedAdd, One, Saturating, StaticLookup};
use sp_std::{convert::TryInto, vec::Vec};
use weights::WeightInfo;

mod benchmarking;
pub mod types;
pub mod weights;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {

	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::OriginFor;

	pub const RESERVE_ID: ReserveIdentifier = ReserveIdentifier::Nft;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_uniques::Config {
		/// Currency type for reserve balance.
		type Currency: NamedReservableCurrency<Self::AccountId, ReserveIdentifier = ReserveIdentifier>;
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// Amount that must be reserved for each minted NFT
		#[pallet::constant]
		type TokenDeposit: Get<BalanceOf<Self>>;
		type WeightInfo: WeightInfo;
	}

	/// Next available class ID.
	#[pallet::storage]
	#[pallet::getter(fn next_class_id)]
	pub type NextClassId<T: Config> = StorageValue<_, T::ClassId, ValueQuery>;

	/// Next available token ID.
	#[pallet::storage]
	#[pallet::getter(fn next_token_id)]
	pub type NextTokenId<T: Config> = StorageMap<_, Twox64Concat, T::ClassId, T::InstanceId, ValueQuery>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Creates an NFT class and sets its metadata
		///
		/// Parameters:
		/// - `class_id`: The identifier of the new asset class. This must not be currently in use.
		/// - `class_type`: The class type determines its purpose and usage
		///
		/// Emits `Created` and `ClassMetadataSet` events when successful.
		#[pallet::weight(<T as Config>::WeightInfo::create_class())]
		#[transactional]
		pub fn create_class(
			origin: OriginFor<T>,
			class_id: T::ClassId,
			class_type: types::ClassType,
		) -> DispatchResult {
			let sender = ensure_signed(origin.clone())?;

			let admin = T::Lookup::unlookup(sender.clone());

			// TODO: finish incremental counter to create class and instance, problem = Class ID not arithmetical
			/* let class_id = NextClassId::<T>::try_mutate(|id| -> Result<T::ClassId, DispatchError> {
				let current_id = *id;
				*id = id.saturating_add(&One::one()).ok_or(Error::<T>::NoAvailableClassId)?;
				Ok(current_id)
			})?; */

			pallet_uniques::Pallet::<T>::create(origin.clone(), class_id, admin.clone())?;

			let key1_bounded = Self::to_bounded_key(b"type".to_vec())?;
			let value1_bounded = Self::to_bounded_value(class_type.encode())?;

			pallet_uniques::Pallet::<T>::set_attribute(origin.clone(), class_id, None, key1_bounded, value1_bounded)?;

			//pallet_uniques::Pallet::<T>::freeze_class(origin.clone(), class_id)?;

			Ok(())
		}

		/// Mints an NFT in the specified class
		/// Sets metadata and the royalty attribute
		///
		/// Parameters:
		/// - `class_id`: The class of the asset to be minted.
		/// - `instance_id`: The instance value of the asset to be minted.
		/// - `owner`: The initial owner of the minted asset.
		///
		/// Emits `Issued` and `AttributeSet` and `MetadataSet` events when successful.
		#[pallet::weight(<T as Config>::WeightInfo::mint())]
		#[transactional]
		pub fn mint(
			origin: OriginFor<T>,
			class_id: T::ClassId,
			instance_id: T::InstanceId,
			ipfs_hash: BoundedVec<u8, T::ValueLimit>,
		) -> DispatchResult {
			let sender = ensure_signed(origin.clone())?;

			let owner = T::Lookup::unlookup(sender.clone());

			let key1_bounded = Self::to_bounded_key(b"ipfs_hash".to_vec())?;

			pallet_uniques::Pallet::<T>::mint(origin.clone(), class_id, instance_id, owner.clone())?;

			pallet_uniques::Pallet::<T>::set_instance_attribute(class_id,
																instance_id,
																key1_bounded.clone(),
																ipfs_hash.clone())?;

			<T as Config>::Currency::reserve_named(&RESERVE_ID, &sender, T::TokenDeposit::get())?;

			// TODO: Increase counter

			Ok(())
		}

		/// Transfers NFT from account A to account B
		/// Only the owner can send their NFT to another account
		///
		/// Parameters:
		/// - `class_id`: The class of the asset to be transferred.
		/// - `instance_id`: The instance of the asset to be transferred.
		/// - `dest`: The account to receive ownership of the asset.
		#[pallet::weight(<T as Config>::WeightInfo::transfer())]
		#[transactional]
		pub fn transfer(
			origin: OriginFor<T>,
			class_id: T::ClassId,
			instance_id: T::InstanceId,
			dest: <T::Lookup as StaticLookup>::Source,
		) -> DispatchResult {
			let sender = ensure_signed(origin.clone())?;

			let dest_account = T::Lookup::lookup(dest.clone())?;

			// Move the deposit to the new owner.
			<T as Config>::Currency::reserve_named(&RESERVE_ID, &dest_account, T::TokenDeposit::get())?;
			<T as Config>::Currency::unreserve_named(&RESERVE_ID, &sender, T::TokenDeposit::get());

			pallet_uniques::Pallet::<T>::transfer(origin, class_id, instance_id, dest)?;

			Ok(())
		}

		/// Removes a token from existence
		///
		/// Parameters:
		/// - `class_id`: The class of the asset to be burned.
		/// - `instance_id`: The instance of the asset to be burned.
		/// - `check_owner`: If `Some` then the operation will fail with `WrongOwner` unless the
		///   asset is owned by this value.
		///
		/// Emits `Burned` with the actual amount burned.
		#[pallet::weight(<T as Config>::WeightInfo::burn())]
		#[transactional]
		pub fn burn(origin: OriginFor<T>, class_id: T::ClassId, instance_id: T::InstanceId) -> DispatchResult {
			let sender = ensure_signed(origin.clone())?;

			ensure!(
				pallet_uniques::Pallet::<T>::can_transfer(&class_id, &instance_id),
				Error::<T>::TokenFrozen
			);

			pallet_uniques::Pallet::<T>::burn(origin, class_id, instance_id, None)?;
			<T as Config>::Currency::unreserve_named(&RESERVE_ID, &sender, T::TokenDeposit::get());

			Ok(())
		}

		/// Removes a class from existence
		///
		/// Parameters:
		/// - `class_id`: The identifier of the asset class to be destroyed.
		///
		/// Emits `Destroyed` event when successful.
		#[pallet::weight(<T as Config>::WeightInfo::destroy_class())]
		pub fn destroy_class(origin: OriginFor<T>, class_id: T::ClassId) -> DispatchResultWithPostInfo {
			ensure_signed(origin.clone())?;

			/* TODO:
			The following does not work, how to check efficiently if there are no instances minted in class?

			ensure!(
				pallet_uniques::Pallet::<T>::instances(&class_id).peekable().peek().is_none(),
				Error::<T>::TokenClassNotEmpty

			); */

			let witness = pallet_uniques::Pallet::<T>::get_destroy_witness(&class_id).ok_or(Error::<T>::NoWitness)?;

			pallet_uniques::Pallet::<T>::destroy(origin, class_id, witness)?;

			Ok(().into())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {}

	#[pallet::error]
	pub enum Error<T> {
		/// String exceeds allowed length
		TooLong,
		/// Count of instances overflown
		NoAvailableInstanceId,
		/// Witness not available
		WitnessUnavailable,
		/// Cannot burn token if frozen
		TokenFrozen,
		/// No witness found for given class
		NoWitness,
		/// Class still contains minted tokens
		TokenClassNotEmpty,
		/// Number of classes reached the limit
		NoAvailableClassId,
	}
}

impl<T: Config> Pallet<T> {
	fn to_bounded_key(name: Vec<u8>) -> Result<BoundedVec<u8, T::KeyLimit>, Error<T>> {
		name.try_into().map_err(|_| Error::<T>::TooLong)
	}

	fn to_bounded_value(name: Vec<u8>) -> Result<BoundedVec<u8, T::ValueLimit>, Error<T>> {
		name.try_into().map_err(|_| Error::<T>::TooLong)
	}
}
