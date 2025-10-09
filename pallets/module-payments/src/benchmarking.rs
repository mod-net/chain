//! Benchmarking setup for pallet-module-payments
#![cfg(feature = "runtime-benchmarks")]
use super::*;

use frame_support::BoundedVec;

#[allow(unused)]
use crate::Pallet as ModulePayments;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn set_authorized_module() {
        let caller: T::AccountId = whitelisted_caller();
        let module: pallet_modules::module::Module<<T as Config>::Modules> =
            pallet_modules::module::Module {
                owner: caller.clone(),
                id: 0,
                name: BoundedVec::try_from("Test Module".as_bytes().to_vec()).expect("too long"),
                data: None,
                url: None,
                collateral: 20,
                take: frame_support::sp_runtime::Percent::zero(),
                tier: pallet_modules::module::ModuleTier::Official,
                created_at: 0,
                last_updated: 0,
            };
        pallet_modules::Modules::insert(0, module);

        #[extrinsic_call]
        ModulePayments::set_authorized_module(RawOrigin::Root, 0u64);
    }

    #[benchmark]
    fn set_module_weights() {
        let caller: T::AccountId = whitelisted_caller();
        let rando_module_1: T::AccountId = account("rando_module_1", 1, 1);
        let rando_module_2: T::AccountId = account("rando_module_2", 2, 2);
        let modules: [pallet_modules::module::Module<<T as Config>::Modules>; 3] = [
            pallet_modules::module::Module {
                owner: caller.clone(),
                id: 0,
                name: BoundedVec::try_from(b"authorized_module".to_vec()).expect("too long"),
                data: None,
                url: None,
                collateral: 20,
                take: frame_support::sp_runtime::Percent::zero(),
                tier: pallet_modules::module::ModuleTier::Official,
                created_at: 0,
                last_updated: 0,
            },
            pallet_modules::module::Module {
                owner: rando_module_1.clone(),
                id: 1,
                name: BoundedVec::try_from(b"rando_module_1".to_vec()).expect("too long"),
                data: None,
                url: None,
                collateral: 20,
                take: frame_support::sp_runtime::Percent::zero(),
                tier: pallet_modules::module::ModuleTier::Approved,
                created_at: 0,
                last_updated: 0,
            },
            pallet_modules::module::Module {
                owner: rando_module_2.clone(),
                id: 2,
                name: BoundedVec::try_from(b"rando_module_2".to_vec()).expect("too long"),
                data: None,
                url: None,
                collateral: 20,
                take: frame_support::sp_runtime::Percent::zero(),
                tier: pallet_modules::module::ModuleTier::Official,
                created_at: 0,
                last_updated: 0,
            },
        ];
        for m in modules.iter() {
            pallet_modules::Modules::insert(&m.id, m);
        }

        assert_eq!(
            ModulePayments::<T>::set_authorized_module(RawOrigin::Root.into(), 0u64),
            Ok(())
        );

        #[extrinsic_call]
        ModulePayments::<T>::set_module_weights(
            RawOrigin::Signed(caller.clone()),
            [1, 2].to_vec(),
            [80, 100].to_vec(),
        );
    }

    #[benchmark]
    fn report_payment() {
        let caller: T::AccountId = whitelisted_caller();
        let rando_module_1: T::AccountId = account("rando_module_1", 1, 1);
        let rando_user_2: T::AccountId = account("rando_user_2", 2, 2);
        drop(<T as crate::Config>::Currency::deposit_creating(
            &rando_user_2,
            10_000_000_000_000,
        ));
        let modules: [pallet_modules::module::Module<<T as Config>::Modules>; 2] = [
            pallet_modules::module::Module {
                owner: caller.clone(),
                id: 0,
                name: BoundedVec::try_from(b"authorized_module".to_vec()).expect("too long"),
                data: None,
                url: None,
                collateral: 20,
                take: frame_support::sp_runtime::Percent::zero(),
                tier: pallet_modules::module::ModuleTier::Official,
                created_at: 0,
                last_updated: 0,
            },
            pallet_modules::module::Module {
                owner: rando_module_1.clone(),
                id: 1,
                name: BoundedVec::try_from(b"rando_module_1".to_vec()).expect("too long"),
                data: None,
                url: None,
                collateral: 20,
                take: frame_support::sp_runtime::Percent::zero(),
                tier: pallet_modules::module::ModuleTier::Approved,
                created_at: 0,
                last_updated: 0,
            },
        ];
        for m in modules.iter() {
            pallet_modules::Modules::insert(&m.id, m);
        }

        assert_eq!(
            ModulePayments::<T>::set_authorized_module(RawOrigin::Root.into(), 0u64),
            Ok(())
        );

        #[extrinsic_call]
        ModulePayments::<T>::report_payment(
            RawOrigin::Signed(caller.clone()),
            crate::PaymentReport {
                module_id: 1u64,
                payee: rando_user_2.clone(),
                amount: 9_999_000_000_000,
            },
        );
    }

    impl_benchmark_test_suite!(
        ModulePayments,
        crate::mock::new_test_ext(),
        crate::mock::Test
    );
}
