#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_core::H256;
    use sp_std::vec::Vec;
    use codec::{Decode, Encode, MaxEncodedLen, DecodeWithMemTracking};

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn merkle_root)]
    pub type MerkleRoot<T> = StorageValue<_, H256, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn snapshot_block)]
    pub type SnapshotBlock<T> = StorageValue<_, u32, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn snapshot_time)]
    pub type SnapshotTime<T> = StorageValue<_, u64, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn base_ratio)]
    pub type BaseRatio<T> = StorageValue<_, u128, OptionQuery>;

    #[derive(Clone, Encode, Decode, DecodeWithMemTracking, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum UnlockShape { Linear, BackLoaded(u8) }

    #[derive(Clone, Encode, Decode, DecodeWithMemTracking, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub struct Params {
        pub t_min_days: u32,
        pub t_max_days: u32,
        pub k_power: u8,
        pub unlock_shape: UnlockShape,
    }

    #[pallet::storage]
    #[pallet::getter(fn params)]
    pub type BridgeParams<T> = StorageValue<_, Params, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn paused)]
    pub type Paused<T> = StorageValue<_, bool, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn claimed)]
    pub type Claimed<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, bool, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        Claimed { account: T::AccountId, base: u128, t_days: u32, effective: u128 },
        ParamsUpdated,
        Paused,
        Unpaused,
    }

    #[pallet::error]
    pub enum Error<T> {
        AlreadyClaimed,
        BridgePaused,
        ParamsNotSet,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(10_000)]
        pub fn claim(origin: OriginFor<T>, _proof: Vec<u8>, _leaf: Vec<u8>, t_days: u32) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(!Paused::<T>::get(), Error::<T>::BridgePaused);
            ensure!(!Claimed::<T>::get(&who), Error::<T>::AlreadyClaimed);
            ensure!(BridgeParams::<T>::get().is_some(), Error::<T>::ParamsNotSet);

            // For skeleton: mark claimed and emit a placeholder calculation
            Claimed::<T>::insert(&who, true);
            let base: u128 = 0;
            let effective: u128 = 0;
            Self::deposit_event(Event::Claimed { account: who, base, t_days, effective });
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(10_000)]
        pub fn set_params(origin: OriginFor<T>, params: Params) -> DispatchResult {
            ensure_root(origin)?;
            BridgeParams::<T>::put(params);
            Self::deposit_event(Event::ParamsUpdated);
            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(10_000)]
        pub fn pause(origin: OriginFor<T>) -> DispatchResult {
            ensure_root(origin)?;
            Paused::<T>::put(true);
            Self::deposit_event(Event::Paused);
            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(10_000)]
        pub fn unpause(origin: OriginFor<T>) -> DispatchResult {
            ensure_root(origin)?;
            Paused::<T>::put(false);
            Self::deposit_event(Event::Unpaused);
            Ok(())
        }
    }

    pub trait WeightInfo { }
    impl WeightInfo for () { }
}
