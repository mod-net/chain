// Modules Pallet
// Responsible for the registry and management of
// Modules and Replicants

#![cfg_attr(not(feature = "std"), no_std)]

mod ext;
pub mod module;
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::*;

pub(crate) use ext::*;
use frame_support::traits::{
    Currency, InspectLockableCurrency, LockableCurrency, NamedReservableCurrency,
};
use frame_system::pallet_prelude::BlockNumberFor;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        dispatch::DispatchResult, pallet_prelude::*, sp_runtime::Percent, traits::ConstU64,
    };
    use frame_system::{ensure_signed, pallet_prelude::*};
    extern crate alloc;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// A type representing the weights required by the extrinsics of this pallet.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type WeightInfo: WeightInfo;
        type Currency: Currency<Self::AccountId, Balance = u128>
            + LockableCurrency<Self::AccountId, Moment = BlockNumberFor<Self>>
            + InspectLockableCurrency<Self::AccountId>
            + NamedReservableCurrency<Self::AccountId, ReserveIdentifier = [u8; 8]>
            + Send
            + Sync;

        /// Maximum number of Modules a single User (Account) can register
        #[pallet::constant]
        type MaxModules: Get<u64>;
        /// Maximum number of Replicants that can be active per Module
        #[pallet::constant]
        type MaxModuleReplicants: Get<u16>;
        /// Maximum Take Percentage a Module Owner can set
        #[pallet::constant]
        type DefaultMaxModuleTake: Get<Percent>;
        /// Maximum length for Module Name String
        #[pallet::constant]
        type MaxModuleNameLength: Get<u32>;
        /// Maximum length for Storage Reference data (in bytes), e.g. IPFS CID, S3 URL, etc.
        #[pallet::constant]
        type MaxStorageReferenceLength: Get<u32>;
        /// Maximum length for a URL
        #[pallet::constant]
        type MaxURLLength: Get<u32>;
        /// Default Module Registration Cost
        #[pallet::constant]
        type DefaultModuleCollateral: Get<u128>;
    }

    #[pallet::storage]
    pub type MaxModuleTake<T: Config> =
        StorageValue<_, Percent, ValueQuery, T::DefaultMaxModuleTake>;

    #[pallet::storage]
    pub type ModuleCollateral<T: Config> =
        StorageValue<_, BalanceOf<T>, ValueQuery, T::DefaultModuleCollateral>;

    #[pallet::storage]
    pub type Modules<T: Config> = StorageMap<_, Identity, u64, module::Module<T>>;

    #[pallet::storage]
    pub type ModuleCount<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    pub type NextModule<T: Config> = StorageValue<_, u64, ValueQuery, ConstU64<0>>;

    /// Events emitted by this pallet.
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A module was successfully registered.
        ModuleRegistered {
            /// The account who registered the module.
            who: T::AccountId,
            /// The ID of the module
            id: u64,
            /// Name of the module
            name: module::ModuleName<T>,
            /// Data reference of the module
            data: StorageReference<T>,
            /// URL of the module
            url: URLReference<T>,
            /// Collateral
            collateral: BalanceOf<T>,
            /// Take Percentage
            take: Percent,
        },
        /// A module was successfully updated.
        ModuleUpdated {
            /// The account who updated the module.
            who: T::AccountId,
            /// The ID of the module
            id: u64,
            /// Name of the module (potentially changed)
            name: module::ModuleName<T>,
            /// Data reference of the module (potentially changed)
            data: StorageReference<T>,
            /// URL of the module
            url: URLReference<T>,
            /// Take of the module (potentially changed)
            take: Percent,
        },
        /// A module was successfully removed.
        ModuleRemoved {
            /// The account who removed the module.
            who: T::AccountId,
            /// The ID of the module
            id: u64,
        },
        ModuleTierChanged {
            id: u64,
            tier: module::ModuleTier,
        },
    }

    /// Errors that can be returned by this pallet.
    #[pallet::error]
    pub enum Error<T> {
        /// Something went wrong that shouldn't have ever gone wrong
        InternalError,
        /// Module Name is not UTF8
        NameNotUTF8,
        /// Module Name is Empty
        NameEmpty,
        /// Module Name is Too Long
        NameLengthExceeded,
        /// Module Name has leading or trailing whitespace
        NameWhitespace,
        /// Name is Taken
        NameTaken,
        /// Max Modules Reached
        MaxModulesReached,
        /// Maximum Take Exceeded
        MaxTakeExceeded,
        /// The module does not exist in the registry.
        ModuleNotFound,
        /// The module is not owned by the caller
        ModuleOwnership,
    }

    /// Dispatchable functions for the module registry pallet.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::register_module())]
        pub fn register_module(
            origin: OriginFor<T>,
            name: module::ModuleName<T>,
            data: StorageReference<T>,
            url: URLReference<T>,
            take: Option<Percent>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            module::register::<T>(who, name, data, url, take)
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::remove_module())]
        pub fn remove_module(origin: OriginFor<T>, id: u64) -> DispatchResult {
            let who = ensure_signed(origin)?;
            module::remove::<T>(who, id)
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::update_module())]
        pub fn update_module(
            origin: OriginFor<T>,
            id: u64,
            name: Option<module::ModuleName<T>>,
            data: StorageReference<T>,
            url: URLReference<T>,
            take: Option<Percent>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            module::update::<T>(who, id, name, data, url, take)
        }

        // TODO: link into voting mechanisms that check for prev usage
        /// Changes the module tier to the specified tier
        #[pallet::call_index(3)]
        #[pallet::weight({0})]
        pub fn change_module_tier(
            origin: OriginFor<T>,
            id: u64,
            tier: module::ModuleTier,
        ) -> DispatchResult {
            ensure_root(origin)?;
            module::change_tier::<T>(id, tier)
        }
    }
}
