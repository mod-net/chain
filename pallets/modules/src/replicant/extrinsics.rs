use super::{ModuleInfo, Replicant};
use crate::{AccountIdOf, URLReference};
use frame_support::{dispatch::DispatchResult, ensure, traits::ReservableCurrency};

pub fn register<T: crate::Config>(
    origin: AccountIdOf<T>,
    module_info: ModuleInfo<T>,
    url: URLReference<T>,
) -> DispatchResult {
    let module_query = crate::Modules::<T>::get(module_info.1);
    match module_query {
        Some(module) => {
            ensure!(
                crate::Replicants::<T>::iter_prefix_values(&module.id)
                    .filter(|r| r.owner == origin)
                    .count()
                    == 0,
                crate::Error::<T>::ReplicantExists,
            );

            let collateral = crate::ReplicantCollateral::<T>::get();
            <T as crate::Config>::Currency::reserve(&origin, collateral)?;

            let current_block: u64 = <frame_system::Pallet<T>>::block_number()
                .try_into()
                .ok()
                .expect("Blocks will not exceed u64 maximum.");

            crate::Replicants::<T>::insert(
                &module.id,
                origin.clone(),
                Replicant {
                    owner: origin.clone(),
                    module: ModuleInfo::<T>(module.owner.clone(), module.id.clone()),
                    url: url.clone(),
                    collateral: collateral.clone(),
                    created_at: current_block,
                    last_updated: current_block,
                },
            );

            crate::Pallet::<T>::deposit_event(crate::Event::<T>::ReplicantRegistered {
                who: origin,
                module: module_info,
                url,
                collateral,
            });

            Ok(())
        }
        None => Err(crate::Error::<T>::ModuleNotFound.into()),
    }
}

pub fn remove<T: crate::Config>(origin: AccountIdOf<T>, module: ModuleInfo<T>) -> DispatchResult {
    let replicant_query = crate::Replicants::<T>::get(module.1, origin.clone());
    match replicant_query {
        Some(replicant) => {
            let collateral = replicant.collateral;
            <T as crate::Config>::Currency::unreserve(&origin, collateral);

            crate::Replicants::<T>::remove(module.1, &origin);

            crate::Pallet::<T>::deposit_event(crate::Event::<T>::ReplicantRemoved {
                who: origin,
                module,
            });

            Ok(())
        }
        None => Err(crate::Error::<T>::ReplicantNotFound.into()),
    }
}

pub fn update<T: crate::Config>(
    origin: AccountIdOf<T>,
    module: ModuleInfo<T>,
    url: URLReference<T>,
) -> DispatchResult {
    crate::Replicants::<T>::try_mutate(module.1, origin.clone(), |replicant_query| -> DispatchResult {
        match replicant_query {
            Some(replicant) => {
                let current_block: u64 = <frame_system::Pallet<T>>::block_number()
                    .try_into()
                    .ok()
                    .expect("Blocks will not exceed u64 maximum.");

                let new_url = url.or(replicant.url.clone());

                replicant.url = new_url.clone();
                replicant.last_updated = current_block;

                crate::Pallet::<T>::deposit_event(crate::Event::<T>::ReplicantUpdated {
                    who: origin,
                    module,
                    url: new_url,
                });

                Ok(())
            }
            None => Err(crate::Error::<T>::ReplicantNotFound.into()),
        }
    })
}
