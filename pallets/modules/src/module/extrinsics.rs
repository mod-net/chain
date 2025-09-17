use super::{Module, ModuleName};
use crate::{AccountIdOf, StorageReference, URLReference};
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
    url: URLReference<T>,
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
    let max_take = crate::MaxModuleTake::<T>::get();
    ensure!(take <= max_take, crate::Error::<T>::MaxTakeExceeded,);

    crate::Modules::<T>::insert(
        next_module_id,
        Module {
            owner: origin.clone(),
            id: next_module_id,
            name: name.clone(),
            data: data.clone(),
            url: url.clone(),
            collateral,
            take,
            created_at: current_block,
            last_updated: current_block,
        },
    );

    crate::NextModule::<T>::mutate(|v| *v = v.saturating_add(1));

    crate::Pallet::<T>::deposit_event(crate::Event::<T>::ModuleRegistered {
        who: origin,
        id: next_module_id,
        name,
        data,
        url,
        collateral,
        take,
    });

    Ok(())
}

pub fn remove<T: crate::Config>(origin: AccountIdOf<T>, id: u64) -> DispatchResult {
    let module_query = crate::Modules::<T>::get(&id);
    match module_query {
        Some(module) => {
            if module.owner != origin.clone() {
                return Err(crate::Error::<T>::ModuleOwnership.into());
            }

            let collateral = module.collateral;
            <T as crate::Config>::Currency::unreserve(&origin, collateral);

            crate::Modules::<T>::remove(&id);

            crate::Pallet::<T>::deposit_event(crate::Event::<T>::ModuleRemoved { who: origin, id });

            Ok(())
        }
        None => Err(crate::Error::<T>::ModuleNotFound.into()),
    }
}

pub fn update<T: crate::Config>(
    origin: AccountIdOf<T>,
    id: u64,
    name: Option<ModuleName<T>>,
    data: StorageReference<T>,
    url: URLReference<T>,
    take: Option<Percent>,
) -> DispatchResult {
    crate::Modules::<T>::try_mutate(&id, |module_query| -> DispatchResult {
        match module_query {
            Some(module) => {
                if module.owner != origin.clone() {
                    return Err(crate::Error::<T>::ModuleOwnership.into());
                }

                let current_block: u64 = <frame_system::Pallet<T>>::block_number()
                    .try_into()
                    .ok()
                    .expect("Blocks will not exceed u64 maximum.");

                let new_name = name.unwrap_or(module.name.clone());
                let new_data = data.or(module.data.clone());
                let new_url = url.or(module.url.clone());
                let new_take = take.unwrap_or(module.take);

                let max_take = crate::MaxModuleTake::<T>::get();
                ensure!(new_take <= max_take, crate::Error::<T>::MaxTakeExceeded);

                module.name = new_name.clone();
                module.data = new_data.clone();
                module.url = new_url.clone();
                module.take = new_take.clone();
                module.last_updated = current_block;

                crate::Pallet::<T>::deposit_event(crate::Event::<T>::ModuleUpdated {
                    who: origin,
                    id,
                    name: new_name,
                    data: new_data,
                    url: new_url,
                    take: new_take,
                });

                Ok(())
            }
            None => Err(crate::Error::<T>::ModuleNotFound.into()),
        }
    })
}
