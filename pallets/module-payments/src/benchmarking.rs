//! Benchmarking setup for pallet-module-payments

use super::*;

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

    #[extrinsic_call]
    set_authorized_module(RawOrigin::Root, 0u64);
  }

  impl_benchmark_test_suite!(
    ModulePayments,
    crate::mock::new_test_ext(),
    crate::mock::Test
  );
}