use crate::{AccountIdOf, BalanceOf, Block, StorageReference};
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use frame_support::{
    sp_runtime::{BoundedVec, Percent},
    dispatch::DispatchResult, ensure, traits::Get,
    DebugNoBound, EqNoBound, PartialEqNoBound,
};
use scale_info::TypeInfo;

pub type ModuleName<T> = BoundedVec<u8, <T as crate::Config>::MaxModuleNameLength>;

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
pub struct Module<T: crate::Config> {
    pub owner: AccountIdOf<T>,
    pub id: u64,
    pub name: ModuleName<T>,
    pub data: StorageReference<T>,
    pub collateral: BalanceOf<T>,
    pub take: Percent,
    pub created_at: Block,
    pub last_updated: Block,
}

impl<T: crate::Config> Module<T> {
    pub fn validate_name(bytes: &[u8]) -> DispatchResult {
        let len: u32 = bytes
            .len()
            .try_into()
            .map_err(|_| crate::Error::<T>::InternalError)?;

        // Name Not UTF8
        ensure!(
            core::str::from_utf8(bytes).is_ok(),
            crate::Error::<T>::NameNotUTF8,
        );

        // Name Empty
        ensure!(
            len > 0,
            crate::Error::<T>::NameEmpty,
        );

        // Exceeds Length
        ensure!(
            len <= T::MaxModuleNameLength::get(),
            crate::Error::<T>::NameLengthExceeded,
        );

        // Leading Whitespace
        ensure!(
            !bytes.first().map_or(false, |b| b.is_ascii_whitespace()),
            crate::Error::<T>::NameWhitespace,
        );

        // Trailing Whitespace
        ensure!(
            !bytes.last().map_or(false, |b| b.is_ascii_whitespace()),
            crate::Error::<T>::NameWhitespace,
        );

        // Name Taken
        ensure!(
            crate::Modules::<T>::iter_values()
                .filter(|k| &k.name[..] == bytes)
                .count()
                == 0,
            crate::Error::<T>::NameTaken,
        );

        Ok(())
    }
}