use crate::{AccountIdOf, BalanceOf, Block, URLReference};
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use frame_support::{DebugNoBound, EqNoBound, PartialEqNoBound};
use scale_info::TypeInfo;

#[derive(
  DebugNoBound,
  Encode,
  Decode,
  DecodeWithMemTracking,
  MaxEncodedLen,
  TypeInfo,
  PartialEqNoBound,
  EqNoBound,
  Clone,
)]
#[scale_info(skip_type_params(T))]
pub struct ModuleInfo<T: crate::Config>(pub AccountIdOf<T>, pub u64);

#[derive(
    DebugNoBound,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    PartialEqNoBound,
    EqNoBound,
)]
#[scale_info(skip_type_params(T))]
pub struct Replicant<T: crate::Config> {
    pub owner: AccountIdOf<T>,
    pub module: ModuleInfo<T>,
    pub url: URLReference<T>,
    pub collateral: BalanceOf<T>,
    pub created_at: Block,
    pub last_updated: Block,
}
