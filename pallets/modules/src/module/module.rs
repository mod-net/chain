use crate::{AccountIdOf, BalanceOf, Block, StorageReference, URLReference};
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use frame_support::{
    dispatch::DispatchResult,
    ensure,
    sp_runtime::{BoundedVec, Percent},
    traits::Get,
    CloneNoBound, DebugNoBound, EqNoBound, PartialEqNoBound,
};
use scale_info::TypeInfo;

pub type ModuleName<T> = BoundedVec<u8, <T as crate::Config>::MaxModuleNameLength>;

/// Module Tier
///
/// This tier structure excludes intentional removal of a module, at which point the
/// module is completely deleted from the chain and can no longer be revived.
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
pub enum ModuleTier {
    /// This module was registered by the chain's administration
    Official,
    /// This module has been approved for general use
    Approved,
    /// This module has not been approved for general use
    Unapproved,
    /// This module has been delisted for lack of use
    Delisted,
}

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
    pub url: URLReference<T>,
    pub collateral: BalanceOf<T>,
    pub take: Percent,
    pub tier: ModuleTier,
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
            crate::Error::<T>::NameNotUTF8
        );

        // Name Empty
        ensure!(len > 0, crate::Error::<T>::NameEmpty);

        // Exceeds Length
        ensure!(
            len <= T::MaxModuleNameLength::get(),
            crate::Error::<T>::NameLengthExceeded
        );

        // Leading Whitespace
        ensure!(
            !bytes.first().map_or(false, |b| b.is_ascii_whitespace()),
            crate::Error::<T>::NameWhitespace
        );

        // Trailing Whitespace
        ensure!(
            !bytes.last().map_or(false, |b| b.is_ascii_whitespace()),
            crate::Error::<T>::NameWhitespace
        );

        // Name Taken
        ensure!(
            crate::Modules::<T>::iter_values()
                .filter(|k| &k.name[..] == bytes)
                .count()
                == 0,
            crate::Error::<T>::NameTaken
        );

        Ok(())
    }
}
