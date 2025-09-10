use super::{Module, ModuleName};
use crate::{AccountIdOf, StorageReference};
use frame_support::{
    dispatch::DispatchResult,
    ensure,
    sp_runtime::Percent,
    traits::{Get, ReservableCurrency},
};

pub fn register<T: crate::Config>(
    origin: AccountIdOf<T>,
    name: ModuleName<T>,
    data: StorageReference<T>,
    take: Option<Percent>,
) -> DispatchResult {
    Module::<T>::validate_name(&name[..])?;
    let collateral = crate::ModuleCollateral::<T>::get();

    <T as crate::Config>::Currency::reserve(&origin, collateral)?;

    let current_block: u64 = <frame_system::Pallet<T>>::block_number()
        .try_into()
        .ok()
        .expect("Blocks will not exceed u64 maximum.");

    let next_module_id = crate::NextModule::<T>::get();

    ensure!(
        next_module_id.saturating_add(1) != <T as crate::Config>::MaxModules::get(),
        crate::Error::<T>::MaxModulesReached,
    );

    let take = take.unwrap_or(Percent::zero());

    crate::Modules::<T>::insert(
        next_module_id,
        Module {
            owner: origin.clone(),
            id: next_module_id,
            name: name.clone(),
            data,
            collateral,
            take,
            created_at: current_block,
            last_updated: current_block,
        },
    );

    crate::NextModule::<T>::mutate(|v| v.saturating_add(1));

    crate::Pallet::<T>::deposit_event(crate::Event::<T>::ModuleRegistered {
        who: origin,
        id: next_module_id,
        name,
        collateral,
        take,
    });
    
    Ok(())
}
