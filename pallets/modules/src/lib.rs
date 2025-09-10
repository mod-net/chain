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
        pallet_prelude::*, sp_runtime::Percent
    };
    use frame_system::{ensure_signed, pallet_prelude::*};
    use sp_core::ConstU64;
    extern crate alloc;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// A type representing the weights required by the extrinsics of this pallet.
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
        type MaxModuleTake: Get<Percent>;
        /// Maximum length for Module Name String
        #[pallet::constant]
        type MaxModuleNameLength: Get<u32>;
        /// Maximum length for Storage Reference data (in bytes), e.g. IPFS CID, S3 URL, etc.
        #[pallet::constant]
        type MaxStorageReferenceLength: Get<u32>;
        /// Default Module Registration Cost
        #[pallet::constant]
        type DefaultModuleCollateral: Get<u128>;
    }

    #[pallet::storage]
    pub type ModuleCollateral<T: Config> = StorageValue<
        _,
        BalanceOf<T>,
        ValueQuery,
        T::DefaultModuleCollateral
    >;

    #[pallet::storage]
    pub type Modules<T: Config> = StorageMap<
        _,
        Identity,
        u64,
        crate::module::Module<T>
    >;

    #[pallet::storage]
    pub type NextModule<T: Config> = StorageValue<
        _,
        u64,
        ValueQuery,
        ConstU64<0>,
    >;

    // /// Storage map for module registry.
    // /// Maps public keys (Vec<u8>) to IPFS CIDs (Vec<u8>).
    // #[pallet::storage]
    // #[pallet::getter(fn modules)]
    // pub type Modules<T: Config> = StorageMap<
    //     _,
    //     Blake2_128Concat,
    //     BoundedVec<u8, T::MaxKeyLength>,
    //     BoundedVec<u8, T::MaxStorageReferenceLength>,
    //     OptionQuery,
    // >;

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
            /// Collateral
            collateral: BalanceOf<T>,
            /// Take Percentage
            take: Percent,
        },
        // /// A module was successfully updated.
        // ModuleUpdated {
        //     /// The public key used as identifier.
        //     key: BoundedVec<u8, T::MaxKeyLength>,
        //     /// The new IPFS CID of the module metadata.
        //     cid: BoundedVec<u8, T::MaxStorageReferenceLength>,
        //     /// The account who updated the module.
        //     who: T::AccountId,
        // },
        // /// A module was successfully removed.
        // ModuleRemoved {
        //     /// The public key used as identifier.
        //     key: BoundedVec<u8, T::MaxKeyLength>,
        //     /// The account who removed the module.
        //     who: T::AccountId,
        // },
    }

    /// Errors that can be returned by this pallet.
    #[pallet::error]
    pub enum Error<T> {
        /// Something went wrong that shouldn't have gone wrong
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
        /// The module does not exist in the registry.
        ModuleNotFound,
        /// The public key format is invalid.
        InvalidKeyFormat,
        /// The IPFS CID format is invalid.
        InvalidCidFormat,
        /// The public key is too long.
        KeyTooLong,
        /// The IPFS CID is too long.
        CidTooLong,
        /// The public key is empty.
        EmptyKey,
        /// The IPFS CID is empty.
        EmptyCid,
        /// The module already exists in the registry.
        ModuleAlreadyExists,
    }

    /// Dispatchable functions for the module registry pallet.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight({0})]
        pub fn register_module(
            origin: OriginFor<T>,
            name: module::ModuleName<T>,
            data: StorageReference<T>,
            take: Option<Percent>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            module::register::<T>(who, name, data, take)
        }
    }
}