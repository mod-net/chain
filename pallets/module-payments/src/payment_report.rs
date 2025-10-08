use crate::{AccountIdOf, BalanceOf};
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen };
use frame_support::{
  CloneNoBound,
  DebugNoBound,
  EqNoBound,
  PartialEqNoBound,
  dispatch::DispatchResult,
  traits::{Currency, ExistenceRequirement},
};
use scale_info::TypeInfo;

#[derive(
  DebugNoBound,
  CloneNoBound,
  Encode,
  Decode,
  DecodeWithMemTracking,
  MaxEncodedLen,
  TypeInfo,
  PartialEqNoBound,
  EqNoBound,
)]
#[scale_info(skip_type_params(T))]
pub struct PaymentReport<T: crate::Config> {
  pub module_id: u64,
  pub payee: AccountIdOf<T>,
  pub amount: BalanceOf<T>,
}

impl<T: crate::Config> PaymentReport<T> {
  pub fn handle(&self) -> DispatchResult {
    if self.amount == 0 {
      return Err(crate::Error::<T>::EmptyPayment.into());
    }

    if let Some(module) = pallet_modules::Modules::<T::Modules>::get(self.module_id) {
      let module_address = module.owner;

      let payee_balance = <T as crate::Config>::Currency::free_balance(&self.payee);
      if payee_balance < self.amount {
        return Err(crate::Error::<T>::InsufficientFunds.into());
      }

      let fee_address = crate::PaymentPoolAddress::<T>::get();
      let payment_fee = crate::ModulePaymentFee::<T>::get();
      let fee_amount = payment_fee.mul_ceil(self.amount);
      let principal = self.amount.saturating_sub(fee_amount);

      <T as crate::Config>::Currency::transfer(
        &self.payee,
        &module_address,
        principal,
        ExistenceRequirement::KeepAlive,
      )?;

      <T as crate::Config>::Currency::transfer(
        &self.payee,
        &fee_address,
        fee_amount,
        ExistenceRequirement::KeepAlive,
      )?;

      crate::Pallet::<T>::deposit_event(crate::Event::<T>::ModulePaymentReported {
        module_id: self.module_id,
        payee: self.payee.clone(),
        amount: self.amount,
        fee: fee_amount,
      });

      Ok(())
    } else {
      Err(pallet_modules::Error::<T::Modules>::ModuleNotFound.into())
    }
  }
}