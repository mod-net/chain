use super::{Module, ModuleName, ModuleTier};
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
    Module::<T>::validate_url(&url)?;

    let current_count = crate::ModuleCount::<T>::get();
    ensure!(
        current_count < <T as crate::Config>::MaxModules::get(),
        crate::Error::<T>::MaxModulesReached
    );

    let new_count = current_count
        .checked_add(1)
        .ok_or(crate::Error::<T>::ModuleCountOverflow)?;

    let take = take.unwrap_or(Percent::zero());
    let max_take = crate::MaxModuleTake::<T>::get();
    ensure!(take <= max_take, crate::Error::<T>::MaxTakeExceeded);

    let collateral = crate::ModuleCollateral::<T>::get();
    <T as crate::Config>::Currency::reserve(&origin, collateral)?;

    let current_block: u64 = <frame_system::Pallet<T>>::block_number()
        .try_into()
        .ok()
        .expect("Blocks will not exceed u64 maximum.");

    let next_module_id = crate::NextModule::<T>::get();

    if let Err(err) = crate::Modules::<T>::try_mutate(&next_module_id, |maybe_module| {
        ensure!(
            maybe_module.is_none(),
            crate::Error::<T>::ModuleAlreadyExists
        );
        *maybe_module = Some(Module {
            owner: origin.clone(),
            id: next_module_id,
            name: name.clone(),
            data: data.clone(),
            url: url.clone(),
            collateral,
            take,
            tier: ModuleTier::Unapproved,
            created_at: current_block,
            last_updated: current_block,
        });
        Ok(())
    }) {
        <T as crate::Config>::Currency::unreserve(&origin, collateral);
        return Err(err);
    }

    crate::ModuleCount::<T>::put(new_count);

    if let Err(err) = crate::NextModule::<T>::try_mutate(|v| -> DispatchResult {
        *v = v
            .checked_add(1)
            .ok_or(crate::Error::<T>::ModuleIdOverflow)?;
        Ok(())
    }) {
        crate::Modules::<T>::remove(&next_module_id);
        crate::ModuleCount::<T>::put(current_count);
        <T as crate::Config>::Currency::unreserve(&origin, collateral);
        return Err(err);
    }

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

            crate::ModuleCount::<T>::try_mutate(|count| -> DispatchResult {
                ensure!(*count > 0, crate::Error::<T>::ModuleCountUnderflow);
                *count -= 1;
                Ok(())
            })?;

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
    data: Option<StorageReference<T>>,
    url: Option<URLReference<T>>,
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
                let new_data = data.unwrap_or_else(|| module.data.clone());
                let new_url = url.unwrap_or_else(|| module.url.clone());
                Module::<T>::validate_url(&new_url)?;

                let new_take = take.unwrap_or(module.take);

                let max_take = crate::MaxModuleTake::<T>::get();
                ensure!(new_take <= max_take, crate::Error::<T>::MaxTakeExceeded);

                module.name = new_name.clone();
                module.data = new_data.clone();
                module.url = new_url.clone();
                module.take = new_take;
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

pub fn change_tier<T: crate::Config>(id: u64, tier: ModuleTier) -> DispatchResult {
    crate::Modules::<T>::try_mutate(&id, |module_query| -> DispatchResult {
        match module_query {
            Some(module) => {
                let current_block: u64 = <frame_system::Pallet<T>>::block_number()
                    .try_into()
                    .ok()
                    .expect("Blocks will not exceed u64 maximum.");

                module.tier = tier.clone();
                module.last_updated = current_block;

                crate::Pallet::<T>::deposit_event(crate::Event::<T>::ModuleTierChanged {
                    id,
                    tier,
                });

                Ok(())
            }
            None => Err(crate::Error::<T>::ModuleNotFound.into()),
        }
    })
}
