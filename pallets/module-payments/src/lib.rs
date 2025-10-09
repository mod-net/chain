// Module Payments Pallet
// Responsible for:
// - The registration of an "Authorized Module" responsible for all payment management
// - The allocation of user funds for module payments
// - The allocation of payment fees
// - The setting of module weights used for payment fee allocations

#![cfg_attr(not(feature = "std"), no_std)]

mod ext;
mod payment_report;
pub use pallet::*;
pub use payment_report::*;

#[cfg(test)]
pub mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::*;

pub(crate) use ext::*;
use frame_support::traits::{
    Currency,
    InspectLockableCurrency,
    LockableCurrency,
    NamedReservableCurrency,
};
use frame_system::pallet_prelude::BlockNumberFor;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        PalletId,
        dispatch::DispatchResult,
        ensure,
        pallet_prelude::*,
        sp_runtime::{ Perbill, traits::AccountIdConversion },
        traits::ConstU64,
    };
    use frame_system::{ ensure_signed, pallet_prelude::* };
    use sp_std::vec::Vec;
    use sp_std::collections::btree_map::BTreeMap;
    extern crate alloc;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type WeightInfo: WeightInfo;
        type Currency: Currency<Self::AccountId, Balance = u128> +
            LockableCurrency<Self::AccountId, Moment = BlockNumberFor<Self>> +
            InspectLockableCurrency<Self::AccountId> +
            NamedReservableCurrency<Self::AccountId, ReserveIdentifier = [u8; 8]> +
            Send +
            Sync;
        type Modules: pallet_modules::Config<AccountId = Self::AccountId>;

        #[pallet::constant]
        type PalletId: Get<PalletId>;

        #[pallet::constant]
        type DefaultModulePaymentFee: Get<Perbill>;

        #[pallet::constant]
        type DefaultPaymentDistributionPeriod: Get<Block>;

        #[pallet::constant]
        type ExistentialDeposit: Get<u128>;

        #[pallet::constant]
        type MaxModules: Get<u64>;
    }

    fn module_weight_percentages<T: Config>() -> BTreeMap<u64, Perbill> {
        ModuleUsageWeights::<T>
            ::iter()
            .map(|(module_id, module_weight)| (
                module_id,
                Perbill::from_rational(u32::from(module_weight), u32::from(u16::MAX)),
            ))
            .collect()
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(block_number: BlockNumberFor<T>) -> Weight {
            let block_number: u64 = block_number
                .try_into()
                .ok()
                .expect("Blocks will never exceed u64 maximum.");
            // let mut total_weight = Weight::zero();
            let mut total_weight = T::DbWeight::get().reads_writes(0, 0);
            let distribution_period = PaymentDistributionPeriod::<T>::get();
            total_weight = total_weight.saturating_add(T::DbWeight::get().reads(1));
            if block_number % distribution_period != 0u64 {
                return total_weight;
            }
            let existential_deposit = T::ExistentialDeposit::get();
            let pool_address = PaymentPoolAddress::<T>::get();
            let pool_balance = <T as crate::Config>::Currency
                ::free_balance(&pool_address)
                .saturating_sub(existential_deposit);
            total_weight = total_weight.saturating_add(T::DbWeight::get().reads(3));

            let module_weight_percentages: BTreeMap<
                u64,
                Perbill
            > = module_weight_percentages::<T>();
            total_weight = total_weight.saturating_add(T::DbWeight::get().reads(1));

            pallet_modules::Modules::<T::Modules>::iter().for_each(|(module_id, module)| {
                let module_weight_percentage = module_weight_percentages.get(&module_id);
                match module_weight_percentage {
                    Some(percentage) => {
                        let fee_to_distribute = percentage.mul_floor(pool_balance);
                        // println!("p: {:?};\n\tf: {:?}", percentage, fee_to_distribute);
                        let module_address = module.owner;
                        let _transfer_result = <T as crate::Config>::Currency::transfer(
                            &pool_address,
                            &module_address,
                            fee_to_distribute,
                            frame_support::traits::ExistenceRequirement::KeepAlive
                        );
                        // println!("transfer_result: {:?}", transfer_result);
                        total_weight = total_weight.saturating_add(
                            T::DbWeight::get().reads_writes(1, 1)
                        );
                    }
                    None => {
                        // println!("percentage was none");
                        total_weight = total_weight.saturating_add(T::DbWeight::get().reads(1));
                    }
                }
            });

            total_weight
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        AuthorizedModuleSet {
            module_id: u64,
        },
        ModulePaymentReported {
            module_id: u64,
            payee: AccountIdOf<T>,
            amount: u128,
            fee: u128,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        NotAuthorizedModule,
        LengthMismatch,
        InsufficientFunds,
        EmptyPayment,
    }

    #[pallet::type_value]
    pub fn DefaultPaymentPoolAddress<T: Config>() -> T::AccountId {
        <T as Config>::PalletId::get().into_account_truncating()
    }

    #[pallet::storage]
    pub type PaymentPoolAddress<T: Config> = StorageValue<
        _,
        T::AccountId,
        ValueQuery,
        DefaultPaymentPoolAddress<T>
    >;

    #[pallet::storage]
    pub type AuthorizedModule<T: Config> = StorageValue<_, u64, ValueQuery, ConstU64<0>>;

    #[pallet::storage]
    pub type ModuleUsageWeights<T: Config> = StorageMap<_, Identity, u64, u16>;

    #[pallet::storage]
    pub type ModulePaymentFee<T: Config> = StorageValue<
        _,
        Perbill,
        ValueQuery,
        T::DefaultModulePaymentFee
    >;

    #[pallet::storage]
    pub type PaymentDistributionPeriod<T: Config> = StorageValue<
        _,
        Block,
        ValueQuery,
        T::DefaultPaymentDistributionPeriod
    >;

    pub fn ensure_authorized_module<T: crate::Config>(
        origin: OriginFor<T>
    ) -> Result<(), frame_support::sp_runtime::DispatchError> {
        let caller = ensure_signed(origin)?;
        let authorized_module_id = crate::AuthorizedModule::<T>::get();
        let authorized_module = pallet_modules::Modules::<T::Modules>
            ::get(authorized_module_id)
            .ok_or(pallet_modules::Error::<T::Modules>::ModuleNotFound)?;
        let authorized = authorized_module.owner == caller;
        if authorized {
            Ok(())
        } else {
            Err(crate::Error::<T>::NotAuthorizedModule.into())
        }
    }

    /// Normalizes weights of [u16]
    pub fn normalize_weights(weights: &[u16]) -> Vec<u16> {
        let sum: u64 = weights
            .iter()
            .map(|&x| u64::from(x))
            .sum();
        if sum == 0 {
            return weights.to_vec();
        }
        weights
            .iter()
            .map(|&x| {
                u64::from(x)
                    .checked_mul(u64::from(u16::MAX))
                    .and_then(|product| product.checked_div(sum))
                    .and_then(|result| result.try_into().ok())
                    .unwrap_or(0)
            })
            .collect()
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::set_authorized_module())]
        pub fn set_authorized_module(origin: OriginFor<T>, module_id: u64) -> DispatchResult {
            ensure_root(origin)?;

            let module_exists = pallet_modules::Modules::<T::Modules>::contains_key(module_id);

            if module_exists {
                crate::AuthorizedModule::<T>::mutate(|v| {
                    *v = module_id;
                });

                crate::Pallet::<T>::deposit_event(crate::Event::<T>::AuthorizedModuleSet {
                    module_id: module_id,
                });

                Ok(())
            } else {
                Err(pallet_modules::Error::<T::Modules>::ModuleNotFound.into())
            }
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::set_module_weights())]
        pub fn set_module_weights(
            origin: OriginFor<T>,
            module_ids: Vec<u64>,
            weights: Vec<u16>
        ) -> DispatchResult {
            ensure_authorized_module::<T>(origin)?;

            ensure!(module_ids.len() == weights.len(), Error::<T>::LengthMismatch);

            let normalized_values = normalize_weights(&weights);
            let desired_pairs: Vec<(u64, u16)> = module_ids
                .into_iter()
                .map(|id| id as u64)
                .zip(normalized_values.into_iter())
                .collect();

            // Build set of desired keys for pruning
            let desired_keys: sp_std::collections::btree_set::BTreeSet<u64> = desired_pairs
                .iter()
                .map(|(k, _)| *k)
                .collect();

            // Remove any existing entries that are no longer desired
            for existing_key in crate::ModuleUsageWeights::<T>::iter_keys() {
                if !desired_keys.contains(&existing_key) {
                    crate::ModuleUsageWeights::<T>::remove(existing_key);
                }
            }

            // Insert/update desired pairs
            for (id, weight) in desired_pairs {
                crate::ModuleUsageWeights::<T>::insert(id, weight);
            }

            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::report_payment())]
        pub fn report_payment(origin: OriginFor<T>, payment: PaymentReport<T>) -> DispatchResult {
            ensure_authorized_module::<T>(origin)?;

            payment.handle()
        }
    }
}
