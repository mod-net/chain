// Module Payments Pallet
// Responsible for:
// - The registration of an "Authorized Module" responsible for all payment management
// - The allocation of user funds for module payments
// - The allocation of payment fees
// - The setting of module weights used for payment fee allocations

#![cfg_attr(not(feature = "std"), no_std)]

mod ext;
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

    #[pallet::config]
    pub trait Config: frame_system::Config {
        // type WeightInfo: WeightInfo;
        type Currency: Currency<Self::AccountId, Balance = u128>
            + LockableCurrency<Self::AccountId, Moment = BlockNumberFor<Self>>
            + InspectLockableCurrency<Self::AccountId>
            + NamedReservableCurrency<Self::AccountId, ReserveIdentifier = [u8; 8]>
            + Send
            + Sync;
        type Modules: pallet_modules::Config<AccountId = Self::AccountId>;

        #[pallet::constant]
        type DefaultModulePaymentFee: Get<Percent>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        AuthorizedModuleSet { module_id: u64 },
    }

    #[pallet::error]
    pub enum Error<T> {}

    #[pallet::storage]
    pub type AuthorizedModule<T: Config> = StorageValue<_, u64, ValueQuery, ConstU64<0>>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight({0})]
        pub fn set_authorized_module(origin: OriginFor<T>, module_id: u64) -> DispatchResult {
            ensure_root(origin)?;

            let module_exists = pallet_modules::Modules::<T::Modules>::contains_key(module_id);

            if module_exists {
                crate::AuthorizedModule::<T>::mutate(|v| *v = module_id);

                crate::Pallet::<T>::deposit_event(crate::Event::<T>::AuthorizedModuleSet {
                    module_id: module_id,
                });

                Ok(())
            } else {
                Err(pallet_modules::Error::<T::Modules>::ModuleNotFound.into())
            }
        }
    }
}
