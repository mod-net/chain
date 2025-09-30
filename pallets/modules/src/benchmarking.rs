//! Benchmarking setup for pallet-module

use super::*;

use frame_support::BoundedVec;

#[allow(unused)]
pub use crate::Pallet as ModulesPallet;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn register_module() {
        let caller: T::AccountId = whitelisted_caller();
        drop(<T as crate::Config>::Currency::deposit_creating(
            &caller,
            1_000_000_000_000_000,
        ));

        #[extrinsic_call]
        register_module(
            RawOrigin::Signed(caller),
            BoundedVec::try_from("Test Module".as_bytes().to_vec()).expect("too long"),
            None,
            None,
            None
        );

        assert!(Modules::<T>::contains_key(0));
    }

    #[benchmark]
    fn update_module() {
        let caller: T::AccountId = whitelisted_caller();
        drop(<T as crate::Config>::Currency::deposit_creating(
            &caller,
            1_000_000_000_000_000,
        ));

        // First register a module
        let _ = ModulesPallet::<T>::register_module(
            RawOrigin::Signed(caller.clone()).into(),
            BoundedVec::try_from("Test Module".as_bytes().to_vec()).expect("too long"),
            None,
            None,
            None,
        );

        #[extrinsic_call]
        update_module(
            RawOrigin::Signed(caller),
            0u64,
            Some(BoundedVec::try_from("Test Module 2".as_bytes().to_vec()).expect("too long")),
            None,
            None,
            None,
        );

        // Verify that the module was updated
        // assert!(Modules::<T>::contains_key(&bounded_key));
    }

    #[benchmark]
    fn remove_module() {
        let caller: T::AccountId = whitelisted_caller();
        drop(<T as crate::Config>::Currency::deposit_creating(
            &caller,
            1_000_000_000_000_000,
        ));

        // First register a module
        let _ = ModulesPallet::<T>::register_module(
            RawOrigin::Signed(caller.clone()).into(),BoundedVec::try_from("Test Module".as_bytes().to_vec()).expect("too long"),
            None,
            None,
            None,
        );

        #[extrinsic_call]
        remove_module(RawOrigin::Signed(caller), 0u64);

        // Verify that the module was removed
        assert!(!Modules::<T>::contains_key(0u64));
    }

    impl_benchmark_test_suite!(
        ModulesPallet,
        crate::mock::new_test_ext(),
        crate::mock::Test
    );
}
