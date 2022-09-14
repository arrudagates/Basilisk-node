pub use basilisk_runtime::{AccountId, VestingPalletId};
use frame_support::{
    construct_runtime, parameter_types,
    traits::{Everything, Nothing},
    weights::{constants::WEIGHT_PER_SECOND, Pays, Weight},
    PalletId,
};
use hydradx_adapters::{MultiCurrencyTrader, ToFeeReceiver};

use orml_xcm_support::{DepositToAlternative, IsNativeConcrete, MultiCurrencyAdapter, MultiNativeAsset};
use sp_runtime::traits::Convert;
use sp_runtime::Perbill;

use codec::{Decode, Encode};
use scale_info::TypeInfo;

use orml_traits::parameter_type_with_key;

use pallet_transaction_multi_payment::{DepositAll, Price, TransferFees};
use polkadot_xcm::latest::prelude::*;
use primitives::Balance;

use frame_system::EnsureRoot;
use pallet_xcm::XcmPassthrough;
use polkadot_parachain::primitives::Sibling;
use primitives::{constants::currency::*, AssetId};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};
use xcm_builder::{
    AccountId32Aliases, AllowUnpaidExecutionFrom, EnsureXcmOrigin, FixedWeightBounds, LocationInverter, ParentIsPreset,
    RelayChainAsNative, SiblingParachainAsNative, SiblingParachainConvertsVia, SignedAccountId32AsNative,
    SignedToAccountId32, SovereignSignedViaLocation,
};
use xcm_executor::{Config, XcmExecutor};
pub type Amount = i128;

pub const ALICE: [u8; 32] = [4u8; 32];
pub const BOB: [u8; 32] = [5u8; 32];
pub const CHARLIE: [u8; 32] = [6u8; 32];
pub const DAVE: [u8; 32] = [7u8; 32];

pub const UNITS: Balance = 1_000_000_000_000;

pub const KARURA_PARA_ID: u32 = 2000;
pub const BASILISK_PARA_ID: u32 = 2090;
pub type BlockNumberKarura = u64;

use cumulus_primitives_core::ParaId;
use frame_support::traits::{GenesisBuild};
use hydradx_traits::pools::SpotPriceProvider;
use orml_currencies::BasicCurrencyAdapter;
use pallet_transaction_payment::TargetedFeeAdjustment;
use polkadot_primitives::v1::{BlockNumber, MAX_CODE_SIZE, MAX_POV_SIZE};
use polkadot_runtime_parachains::configuration::HostConfiguration;
use polkadot_xcm::prelude::MultiLocation;
use sp_arithmetic::FixedU128;
use sp_runtime::traits::AccountIdConversion;

use basilisk_runtime::{AdjustmentVariable, MinimumMultiplier, TargetBlockFullness, WeightToFee};
use xcm_emulator::{decl_test_network, decl_test_parachain, decl_test_relay_chain};

parameter_types! {
	pub const NativeExistentialDeposit: u128 = NATIVE_EXISTENTIAL_DEPOSIT;
		pub const TransactionByteFee: Balance = 10 * MILLICENTS;

}

pub const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);


//Setting up mock for Karura runtime
//TODO: once it works properly, extract it to dedicated file/project
parameter_types! {
	pub const BlockHashCount: u32 = 250;
	pub const SS58Prefix: u8 = 63;
	pub static MockBlockNumberProvider: u64 = 0;
	pub const MaxLocks: u32 = 50;
	pub const ExistentialDeposit: u128 = 500;
	pub const MaxReserves: u32 = 50;
	pub const RelayNetwork: NetworkId = NetworkId::Kusama;
	pub const UnitWeightCost: Weight = 10;
	pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
	pub const MaxInstructions: u32 = 100;
	pub Ancestry: MultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();
	pub BlockLength: frame_system::limits::BlockLength =
		frame_system::limits::BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
}
parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: AssetId| -> Balance {
		1u128
	};
}

impl frame_system::Config for KaruraRuntime {
    type BaseCallFilter = Everything;
    type BlockWeights = ();
    type BlockLength = BlockLength;
    type DbWeight = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = BlockNumberKarura;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u128>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

pub type LocalOriginToLocation = SignedToAccountId32<Origin, AccountId, RelayNetwork>;

/// The means for routing XCM messages which are not for local execution into
/// the right message queues.
pub type XcmRouter = (
    // Two routers - use UMP to communicate with the relay chain:
    cumulus_primitives_utility::ParentAsUmp<ParachainSystem, ()>,
    // ..and XCMP to communicate with the sibling chains.
    XcmpQueue,
);

parameter_types! {
	pub const TreasuryPalletId: PalletId = PalletId(*b"aca/trsy");
	pub KaruraTreasuryAccount: AccountId = TreasuryPalletId::get().into_account();
	pub KsmPerSecond: (AssetId, u128) = (0, 10);

	pub BaseRate: u128 = 100;
}

pub type LocalAssetTransactor = MultiCurrencyAdapter<
    Currencies,
    UnknownTokens,
    IsNativeConcrete<AssetId, CurrencyIdConvert>,
    AccountId,
    LocationToAccountId,
    AssetId,
    CurrencyIdConvert,
    DepositToAlternative<KaruraTreasuryAccount, Currencies, AssetId, AccountId, Balance>,
>;
pub type XcmOriginToCallOrigin = (
    SovereignSignedViaLocation<LocationToAccountId, Origin>,
    RelayChainAsNative<RelayChainOrigin, Origin>,
    SiblingParachainAsNative<cumulus_pallet_xcm::Origin, Origin>,
    SignedAccountId32AsNative<RelayNetwork, Origin>,
    XcmPassthrough<Origin>,
);
pub type LocationToAccountId = (
    ParentIsPreset<AccountId>,
    SiblingParachainConvertsVia<Sibling, AccountId>,
    AccountId32Aliases<RelayNetwork, AccountId>,
);
pub type Barrier = AllowUnpaidExecutionFrom<Everything>;

pub struct XcmConfig;
impl Config for XcmConfig {
    type Call = Call;
    type XcmSender = XcmRouter;
    type AssetTransactor = LocalAssetTransactor;
    type OriginConverter = XcmOriginToCallOrigin;
    type IsReserve = MultiNativeAsset;
    type IsTeleporter = ();
    type LocationInverter = LocationInverter<Ancestry>;
    type Barrier = Barrier;
    type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
    type Trader = MultiCurrencyTrader<
        AssetId,
        Balance,
        Price,
        WeightToFee,
        MultiTransactionPayment,
        CurrencyIdConvert,
        ToFeeReceiver<
            AccountId,
            AssetId,
            Balance,
            Price,
            CurrencyIdConvert,
            DepositAll<KaruraRuntime>,
            MultiTransactionPayment,
        >,
    >;
    type ResponseHandler = ();
    type AssetTrap = ();
    type AssetClaims = ();
    type SubscriptionService = ();
}

pub const CORE_ASSET_ID: AssetId = 0;

parameter_types! {
	pub const ReservedXcmpWeight: Weight = WEIGHT_PER_SECOND / 4;
	pub const ReservedDmpWeight: Weight = WEIGHT_PER_SECOND / 4;
	pub RegistryStringLimit: u32 = 100;
	pub const NativeAssetId : AssetId = CORE_ASSET_ID;

}

impl cumulus_pallet_parachain_system::Config for KaruraRuntime {
    type Event = Event;
    type OnSystemEvent = ();
    type SelfParaId = ParachainInfo;
    type DmpMessageHandler = DmpQueue;
    type ReservedDmpWeight = ReservedDmpWeight;
    type OutboundXcmpMessageSource = XcmpQueue;
    type XcmpMessageHandler = XcmpQueue;
    type ReservedXcmpWeight = ReservedXcmpWeight;
}

impl cumulus_pallet_xcmp_queue::Config for KaruraRuntime {
    type Event = Event;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type ChannelInfo = ParachainSystem;
    type VersionWrapper = ();
    type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
    type ControllerOrigin = EnsureRoot<AccountId>;
    type ControllerOriginConverter = XcmOriginToCallOrigin;
}

impl pallet_xcm::Config for KaruraRuntime {
    type Event = Event;
    type SendXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
    type XcmRouter = XcmRouter;
    type ExecuteXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
    type XcmExecuteFilter = Everything;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type XcmTeleportFilter = Nothing;
    type XcmReserveTransferFilter = Everything;
    type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
    type LocationInverter = LocationInverter<Ancestry>;
    type Origin = Origin;
    type Call = Call;
    const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
    type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
}

impl cumulus_pallet_dmp_queue::Config for KaruraRuntime {
    type Event = Event;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
}

impl parachain_info::Config for KaruraRuntime {}

impl orml_tokens::Config for KaruraRuntime {
    type Event = Event;
    type Balance = Balance;
    type Amount = Amount;
    type CurrencyId = AssetId;
    type WeightInfo = ();
    type ExistentialDeposits = ExistentialDeposits;
    type OnDust = ();
    type MaxLocks = MaxLocks;
    type DustRemovalWhitelist = Nothing;
    type OnNewTokenAccount = ();
    type OnKilledTokenAccount = ();
}

impl pallet_balances::Config for KaruraRuntime {
    type Balance = Balance;
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = frame_system::Pallet<KaruraRuntime>;
    type MaxLocks = ();
    type WeightInfo = ();
    type MaxReserves = MaxReserves;
    type ReserveIdentifier = ();
}
impl cumulus_pallet_xcm::Config for KaruraRuntime {
    type Event = Event;
    type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl pallet_asset_registry::Config for KaruraRuntime {
    type Event = Event;
    type AssetId = AssetId;
    type RegistryOrigin = EnsureRoot<AccountId>;
    type Balance = Balance;
    type AssetNativeLocation = AssetLocation;
    type StringLimit = RegistryStringLimit;
    type NativeAssetId = KaruraNativeCurrencyId;
    type WeightInfo = ();
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<KaruraRuntime>;
type Block = frame_system::mocking::MockBlock<KaruraRuntime>;

#[derive(Debug, Encode, Decode, Clone, PartialEq, Eq, TypeInfo)]
pub struct AssetLocation(pub MultiLocation);

impl Default for AssetLocation {
    fn default() -> Self {
        AssetLocation(MultiLocation::here())
    }
}

pub struct CurrencyIdConvert;

impl Convert<AssetId, Option<MultiLocation>> for CurrencyIdConvert {
    fn convert(id: AssetId) -> Option<MultiLocation> {
        match id {
            CORE_ASSET_ID => Some(MultiLocation::new(
                1,
                X2(Parachain(ParachainInfo::parachain_id().into()), GeneralIndex(id.into())),
            )),
            _ => {
                if let Some(loc) = AssetRegistry::asset_to_location(id) {
                    Some(loc.0)
                } else {
                    None
                }
            }
        }
    }
}

impl Convert<MultiLocation, Option<AssetId>> for CurrencyIdConvert {
    fn convert(location: MultiLocation) -> Option<AssetId> {
        match location {
            MultiLocation {
                parents,
                interior: X2(Parachain(id), GeneralIndex(index)),
            } if parents == 1
                && ParaId::from(id) == ParachainInfo::parachain_id()
                && (index as u32) == CORE_ASSET_ID =>
                {
                    // Handling native asset for this parachain
                    Some(CORE_ASSET_ID)
                }
            // handle reanchor canonical location: https://github.com/paritytech/polkadot/pull/4470
            MultiLocation {
                parents: 0,
                interior: X1(GeneralIndex(index)),
            } if (index as u32) == CORE_ASSET_ID => Some(CORE_ASSET_ID),
            // delegate to asset-registry
            _ => AssetRegistry::location_to_asset(AssetLocation(location)),
        }
    }
}

impl Convert<MultiAsset, Option<AssetId>> for CurrencyIdConvert {
    fn convert(asset: MultiAsset) -> Option<AssetId> {
        if let MultiAsset {
            id: Concrete(location), ..
        } = asset
        {
            Self::convert(location)
        } else {
            None
        }
    }
}

pub struct AccountIdToMultiLocation;
impl Convert<AccountId, MultiLocation> for AccountIdToMultiLocation {
    fn convert(account: AccountId) -> MultiLocation {
        X1(AccountId32 {
            network: NetworkId::Any,
            id: account.into(),
        })
            .into()
    }
}

parameter_types! {
	pub SelfLocation: MultiLocation = MultiLocation::new(1, X1(Parachain(ParachainInfo::parachain_id().into())));
	pub const BaseXcmWeight: Weight = 100_000_000;
	pub const MaxAssetsForTransfer: usize = 2;
}

impl orml_xtokens::Config for KaruraRuntime {
    type Event = Event;
    type Balance = Balance;
    type CurrencyId = AssetId;
    type CurrencyIdConvert = CurrencyIdConvert;
    type AccountIdToMultiLocation = AccountIdToMultiLocation;
    type SelfLocation = SelfLocation;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type Weigher = FixedWeightBounds<BaseXcmWeight, Call, MaxInstructions>;
    type BaseXcmWeight = BaseXcmWeight;
    type LocationInverter = LocationInverter<Ancestry>;
    type MaxAssetsForTransfer = MaxAssetsForTransfer;
}

parameter_types! {
	pub KaruraNativeCurrencyId: AssetId = 0;
	pub const MultiPaymentCurrencySetFee: Pays = Pays::Yes;
}

impl orml_currencies::Config for KaruraRuntime {
    type Event = Event;
    type MultiCurrency = Tokens;
    type NativeCurrency = BasicCurrencyAdapter<KaruraRuntime, Balances, Amount, u32>;
    type GetNativeCurrencyId = KaruraNativeCurrencyId;
    type WeightInfo = ();
}

impl orml_unknown_tokens::Config for KaruraRuntime {
    type Event = Event;
}
pub type SlowAdjustingFeeUpdate<R> =
TargetedFeeAdjustment<R, TargetBlockFullness, AdjustmentVariable, MinimumMultiplier>;

impl pallet_transaction_payment::Config for KaruraRuntime {
    type OnChargeTransaction = TransferFees<Currencies, MultiTransactionPayment, DepositAll<KaruraRuntime>>;
    type TransactionByteFee = TransactionByteFee;
    type OperationalFeeMultiplier = ();
    type WeightToFee = WeightToFee;
    type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
}

pub struct KaruraSpotPriceProviderStub;

impl SpotPriceProvider<AssetId> for KaruraSpotPriceProviderStub {
    type Price = FixedU128;

    fn pair_exists(_asset_a: AssetId, _asset_b: AssetId) -> bool {
        true
    }

    fn spot_price(_asset_a: AssetId, _asset_b: AssetId) -> Option<Self::Price> {
        Some(FixedU128::from_inner(462_962_963_000_u128))
    }
}

impl pallet_transaction_multi_payment::Config for KaruraRuntime {
    type Event = Event;
    type AcceptedCurrencyOrigin = EnsureRoot<AccountId>;
    type Currencies = Currencies;
    type SpotPriceProvider = KaruraSpotPriceProviderStub;
    type WeightInfo = ();
    type WithdrawFeeForSetCurrency = MultiPaymentCurrencySetFee;
    type WeightToFee = WeightToFee;
    type NativeAssetId = KaruraNativeCurrencyId;
    type FeeReceiver = KaruraTreasuryAccount;
}

construct_runtime!(
	pub enum KaruraRuntime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
		{
			System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
			Tokens: orml_tokens::{Pallet, Call, Storage, Event<T>},
			Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
			Currencies: orml_currencies::{Pallet, Event<T>},
			ParachainSystem: cumulus_pallet_parachain_system::{Pallet, Call, Storage, Inherent, Config, Event<T>},
			ParachainInfo: parachain_info::{Pallet, Storage, Config},
			XcmpQueue: cumulus_pallet_xcmp_queue::{Pallet, Call, Storage, Event<T>},
			DmpQueue: cumulus_pallet_dmp_queue::{Pallet, Call, Storage, Event<T>},
			CumulusXcm: cumulus_pallet_xcm::{Pallet, Event<T>, Origin},
			PolkadotXcm: pallet_xcm::{Pallet, Call, Event<T>, Origin},
			AssetRegistry: pallet_asset_registry::{Pallet, Storage, Event<T>},
			XTokens: orml_xtokens::{Pallet, Storage, Call, Event<T>} = 154,
			UnknownTokens: orml_unknown_tokens::{Pallet, Storage, Event} = 155,
			MultiTransactionPayment: pallet_transaction_multi_payment::{Pallet, Call, Config<T>, Storage, Event<T>} = 106,
		}
);
