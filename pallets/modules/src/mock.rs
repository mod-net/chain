use crate as pallet_modules;
use frame_support::{
    derive_impl, parameter_types,
    traits::{ConstU16, ConstU32, ConstU64, ConstU128, VariantCountOf},
};
use sp_core::H256;
use sp_runtime::{
    BuildStorage, Percent, traits::{BlakeTwo256, IdentityLookup}
};

type Block = frame_system::mocking::MockBlock<Test>;

/// Balance of an account.
pub type Balance = u128;

/// Existential deposit.
pub const EXISTENTIAL_DEPOSIT: Balance = 1_000_000_000;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test
    {
        System: frame_system,
        Balances: pallet_balances,
        ModuleRegistry: pallet_modules,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Nonce = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = ConstU64<250>;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ConstU16<42>;
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}


#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig as pallet_balances::DefaultConfig)]
impl pallet_balances::Config for Test {
    type MaxLocks = ConstU32<50>;
    type MaxReserves = ConstU32<50>;
    type ReserveIdentifier = [u8; 8];
    /// The type for recording an account's balance.
    type Balance = Balance;
    /// The ubiquitous event type.
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
    type AccountStore = System;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Test>;
    type FreezeIdentifier = RuntimeFreezeReason;
    type MaxFreezes = VariantCountOf<RuntimeFreezeReason>;
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type DoneSlashHandler = ();
}

parameter_types! {
    // Keep small to test MaxModulesReached edge case easily.
    pub const MaxModules: u64 = 3;
    pub const MaxModuleReplicants: u16 = u16::MAX;
    pub const DefaultMaxModuleTake: Percent = Percent::from_percent(5);
    pub const MaxModuleNameLength: u32 = 78;
    pub const MaxStorageReferenceLength: u32 = 128;
    pub const DefaultModuleCollateral: u128 = 5_000_000;
}

impl pallet_modules::Config for Test {
    type Currency = Balances;
    type WeightInfo = ();

    type MaxModules = MaxModules;
    type MaxModuleReplicants = MaxModuleReplicants;
    type DefaultMaxModuleTake = DefaultMaxModuleTake;
    type MaxModuleNameLength = MaxModuleNameLength;
    type MaxStorageReferenceLength = MaxStorageReferenceLength;
    type DefaultModuleCollateral = DefaultModuleCollateral;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    // Endow some accounts with ample balance for reservation tests.
    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (1, 10_000_000_000_000),
            (2, 10_000_000_000_000),
            (3, 10_000_000_000_000),
        ],
        dev_accounts: Default::default(),
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}
