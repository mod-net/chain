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
        let module: pallet_modules::module::Module<<T as Config>::Modules> = pallet_modules::module::Module {
          owner: caller.clone(),
          id: 0,
          name: BoundedVec::try_from("Test Module".as_bytes().to_vec()).expect("too long"),
          data: None,
          url: None,
          collateral: 20,
          take: frame_support::sp_runtime::Percent::zero(),
          created_at: 0,
          last_updated: 0,
        };
        pallet_modules::Modules::insert(0, module);

        #[extrinsic_call]
        ModulePayments::set_authorized_module(RawOrigin::Root, 0u64);
    }

    impl_benchmark_test_suite!(ModulePayments, crate::mock::new_test_ext(), crate::mock::Test);
}
