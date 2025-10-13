use crate::{mock::*, Error, Event, Pallet as ModulePayments, PaymentReport};
use frame_support::{
    assert_noop, assert_ok, sp_runtime::traits::Get, weights::constants::RocksDbWeight, BoundedVec,
};
extern crate alloc;
use pallet_modules::module::{Module, ModuleTier};

fn bv(input: &[u8]) -> BoundedVec<u8, <Test as pallet_modules::Config>::MaxModuleNameLength> {
    BoundedVec::try_from(input.to_vec()).expect("within bound")
}

#[test]
fn set_authorized_module() {
    new_test_ext().execute_with(|| {
        let not_sudo: u64 = 1;
        assert_ok!(pallet_modules::Pallet::<Test>::register_module(
            RuntimeOrigin::signed(not_sudo),
            bv(b"authorized_module"),
            None,
            None,
            None
        ));

        // Fails if not sudo
        assert_noop!(
            ModulePayments::<Test>::set_authorized_module(RuntimeOrigin::signed(not_sudo), 0u64),
            sp_runtime::DispatchError::BadOrigin
        );

        // Fails if module doesn't exist
        assert_noop!(
            ModulePayments::<Test>::set_authorized_module(RuntimeOrigin::root(), 1u64),
            pallet_modules::Error::<Test>::ModuleNotFound
        );

        // Succeeds if sudo
        assert_ok!(ModulePayments::<Test>::set_authorized_module(
            RuntimeOrigin::root(),
            0u64
        ));

        System::assert_last_event((Event::AuthorizedModuleSet { module_id: 0u64 }).into());
    })
}

#[test]
fn set_module_weights() {
    new_test_ext().execute_with(|| {
        let modules: [Module<Test>; 3] = [
            Module {
                owner: 0,
                id: 0,
                name: bv(b"authorized_module"),
                data: None,
                url: None,
                collateral: 20,
                take: frame_support::sp_runtime::Percent::zero(),
                tier: ModuleTier::Official,
                created_at: 0,
                last_updated: 0,
            },
            Module {
                owner: 1,
                id: 1,
                name: bv(b"rando_module_1"),
                data: None,
                url: None,
                collateral: 20,
                take: frame_support::sp_runtime::Percent::zero(),
                tier: ModuleTier::Approved,
                created_at: 0,
                last_updated: 0,
            },
            Module {
                owner: 2,
                id: 2,
                name: bv(b"rando_module_2"),
                data: None,
                url: None,
                collateral: 20,
                take: frame_support::sp_runtime::Percent::zero(),
                tier: ModuleTier::Official,
                created_at: 0,
                last_updated: 0,
            },
        ];
        for m in modules.iter() {
            pallet_modules::Modules::insert(&m.id, m);
        }

        // Authorized module gets set
        assert_ok!(ModulePayments::<Test>::set_authorized_module(
            RuntimeOrigin::root(),
            0u64
        ));

        // Fail if lengths don't match
        assert_noop!(
            ModulePayments::<Test>::set_module_weights(
                RuntimeOrigin::signed(0u64),
                [1].to_vec(),
                [80, 100].to_vec()
            ),
            Error::<Test>::LengthMismatch
        );

        // Fail if not authorized module
        assert_noop!(
            ModulePayments::<Test>::set_module_weights(
                RuntimeOrigin::signed(1u64),
                [1, 2].to_vec(),
                [80, 100].to_vec()
            ),
            Error::<Test>::NotAuthorizedModule
        );

        // Succeed with correct parameters and status
        assert_ok!(ModulePayments::<Test>::set_module_weights(
            RuntimeOrigin::signed(0u64),
            [1, 2].to_vec(),
            [80, 100].to_vec()
        ));

        let weight_values: Vec<u16> = crate::ModuleUsageWeights::<Test>::iter_values()
            .into_iter()
            .collect();
        assert_eq!(weight_values, [29126u16, 36408u16]);
    })
}

#[test]
fn report_payment() {
    new_test_ext().execute_with(|| {
        let modules: [Module<Test>; 2] = [
            Module {
                owner: 0,
                id: 0,
                name: bv(b"authorized_module"),
                data: None,
                url: None,
                collateral: 20,
                take: frame_support::sp_runtime::Percent::zero(),
                tier: ModuleTier::Official,
                created_at: 0,
                last_updated: 0,
            },
            Module {
                owner: 1,
                id: 1,
                name: bv(b"rando_module_1"),
                data: None,
                url: None,
                collateral: 20,
                take: frame_support::sp_runtime::Percent::zero(),
                tier: ModuleTier::Approved,
                created_at: 0,
                last_updated: 0,
            },
        ];
        for m in modules.iter() {
            pallet_modules::Modules::insert(&m.id, m);
        }
        // Authorized module gets set
        assert_ok!(ModulePayments::<Test>::set_authorized_module(
            RuntimeOrigin::root(),
            0u64
        ));

        // Reported payment fails if not authorized module
        assert_noop!(
            ModulePayments::<Test>::report_payment(
                RuntimeOrigin::signed(1u64),
                PaymentReport {
                    module_id: 1u64,
                    payee: 2u64,
                    amount: 10_000_000_000_000,
                }
            ),
            Error::<Test>::NotAuthorizedModule
        );

        // Reported payment fails if payment is nothing
        assert_noop!(
            ModulePayments::<Test>::report_payment(
                RuntimeOrigin::signed(0u64),
                PaymentReport {
                    module_id: 1u64,
                    payee: 2u64,
                    amount: 0,
                }
            ),
            Error::<Test>::EmptyPayment
        );

        // Reported payment fails if payee balance isn't enough
        assert_noop!(
            ModulePayments::<Test>::report_payment(
                RuntimeOrigin::signed(0u64),
                PaymentReport {
                    module_id: 1u64,
                    payee: 2u64,
                    amount: 50_000_000_000_000,
                }
            ),
            Error::<Test>::InsufficientFunds
        );

        // Reported payment fails if payee balance isn't enough to retain an existential balance
        assert_noop!(
            ModulePayments::<Test>::report_payment(
                RuntimeOrigin::signed(0u64),
                PaymentReport {
                    module_id: 1u64,
                    payee: 2u64,
                    amount: 10_000_000_000_000,
                }
            ),
            Error::<Test>::InsufficientFunds
        );

        // Succeeds otherwise
        assert_ok!(ModulePayments::<Test>::report_payment(
            RuntimeOrigin::signed(0u64),
            PaymentReport {
                module_id: 1u64,
                payee: 2u64,
                amount: 9_999_000_000_000,
            }
        ));

        System::assert_last_event(
            (Event::ModulePaymentReported {
                module_id: 1u64,
                payee: 2u64,
                amount: 9_999_000_000_000,
                fee: 249_975_000_000,
            })
            .into(),
        );
    })
}

#[test]
fn fee_distribution() {
    use frame_support::traits::{Currency, Hooks};

    new_test_ext().execute_with(|| {
        // Setup: Register 3 modules with different owners
        let module_names = [b"mod_a", b"mod_b", b"mod_c"];
        let owners = [1u64, 2u64, 3u64];
        for (_, (owner, &name)) in owners.iter().zip(module_names.iter()).enumerate() {
            assert_ok!(
                pallet_modules::Pallet::<Test>::register_module(
                    RuntimeOrigin::signed(*owner),
                    bv(name),
                    None,
                    None,
                    None
                )
            );
        }

        // Set authorized module to the first module (id 0)
        assert_ok!(ModulePayments::<Test>::set_authorized_module(RuntimeOrigin::root(), 0u64));

        // Set module weights: 0: 10, 1: 20, 2: 70
        let module_ids = vec![0u64, 1u64, 2u64];
        let weights = vec![10u16, 20u16, 70u16];
        assert_ok!(
            ModulePayments::<Test>::set_module_weights(
                RuntimeOrigin::signed(1u64), // owner of module 0
                module_ids.clone(),
                weights.clone()
            )
        );

        // Fund the payment pool address
        let pool_address = crate::PaymentPoolAddress::<Test>::get();
        let pool_balance = 1_000_000_000_000u128;
        // Give pool address some funds
        drop(<Test as crate::Config>::Currency::deposit_creating(&pool_address, pool_balance));

        // Check initial balances
        let bal_1 = Balances::free_balance(&1u64);
        let bal_2 = Balances::free_balance(&2u64);
        let bal_3 = Balances::free_balance(&3u64);
        let existential_deposit = <Test as crate::Config>::ExistentialDeposit::get();
        let bal_pool = Balances::free_balance(&pool_address).saturating_sub(existential_deposit);

        // Simulate block production up to the distribution period
        let period = crate::PaymentDistributionPeriod::<Test>::get();
        let mut block_number = 1u64;
        while block_number < period {
            System::set_block_number(block_number);
            // Should not distribute yet
            let weight = ModulePayments::<Test>::on_initialize(block_number);
            let expected_weight = RocksDbWeight::get().reads_writes(1, 0);
            assert_eq!(weight, expected_weight);
            block_number += 1;
        }

        // At the distribution period, distribution should occur but only up to the per-block limit
        System::set_block_number(period);
        let weight = ModulePayments::<Test>::on_initialize(period);
        let expected_weight = RocksDbWeight::get().reads_writes(3, 3);
        assert_eq!(weight, expected_weight);

        let normalized_weights = crate::normalize_weights(&weights);
        let perbill_0 = sp_runtime::Perbill::from_rational(normalized_weights[0] as u32, u16::MAX as u32);
        let perbill_1 = sp_runtime::Perbill::from_rational(normalized_weights[1] as u32, u16::MAX as u32);
        let perbill_2 = sp_runtime::Perbill::from_rational(normalized_weights[2] as u32, u16::MAX as u32);

        // First block processes two modules
        let payout_first_0 = perbill_0.mul_floor(bal_pool);
        let payout_first_1 = perbill_1.mul_floor(bal_pool);

        let cursor_after_first = crate::PendingPayoutCursor::<Test>::get();
        assert_eq!(cursor_after_first, Some(2));

        let bal_1_after_first = Balances::free_balance(&1u64);
        let bal_2_after_first = Balances::free_balance(&2u64);
        let bal_3_after_first = Balances::free_balance(&3u64);
        let bal_pool_after_first = Balances::free_balance(&pool_address)
            .saturating_sub(existential_deposit);

        assert_eq!(bal_1_after_first, bal_1 + payout_first_0);
        assert_eq!(bal_2_after_first, bal_2 + payout_first_1);
        assert_eq!(bal_3_after_first, bal_3);
        assert_eq!(bal_pool_after_first, bal_pool - payout_first_0 - payout_first_1);

        // Next block completes the remaining payouts
        System::set_block_number(period + 1);
        ModulePayments::<Test>::on_initialize(period + 1);

        let cursor_after_second = crate::PendingPayoutCursor::<Test>::get();
        assert_eq!(cursor_after_second, None);

        let payout_second_2 = perbill_2.mul_floor(bal_pool_after_first);

        let bal_1_final = Balances::free_balance(&1u64);
        let bal_2_final = Balances::free_balance(&2u64);
        let bal_3_final = Balances::free_balance(&3u64);
        let bal_pool_final = Balances::free_balance(&pool_address)
            .saturating_sub(existential_deposit);

        assert_eq!(bal_1_final, bal_1_after_first);
        assert_eq!(bal_2_final, bal_2_after_first);
        assert_eq!(bal_3_final, bal_3 + payout_second_2);
        assert_eq!(bal_pool_final, bal_pool_after_first - payout_second_2);
    });
}

#[test]
fn payouts_respect_max_per_block() {
    use frame_support::traits::{Currency, Hooks};

    new_test_ext().execute_with(|| {
        // Register modules
        for (id, owner) in (0u64..4).zip([1u64, 2u64, 3u64, 4u64].into_iter()) {
            assert_ok!(pallet_modules::Pallet::<Test>::register_module(
                RuntimeOrigin::signed(owner),
                bv(format!("module-{id}").as_bytes()),
                None,
                None,
                None
            ));
        }

        // Authorize module 0
        assert_ok!(ModulePayments::<Test>::set_authorized_module(RuntimeOrigin::root(), 0));

        // Assign equal weights to four modules
        let module_ids = vec![0u64, 1u64, 2u64, 3u64];
        let weights = vec![u16::MAX / 4; 4];
        assert_ok!(ModulePayments::<Test>::set_module_weights(
            RuntimeOrigin::signed(1u64),
            module_ids.clone(),
            weights.clone()
        ));

        // Seed payment pool
        let pool_address = crate::PaymentPoolAddress::<Test>::get();
        let initial_pool = 1_000_000_000_000u128;
        drop(<Test as crate::Config>::Currency::deposit_creating(&pool_address, initial_pool));

        // Capture initial balances
        let mut balances_before = [0u128; 5];
        for account in 1u64..=4u64 {
            balances_before[account as usize] = Balances::free_balance(&account);
        }

        // Trigger payout cycle at distribution period
        let period = crate::PaymentDistributionPeriod::<Test>::get();
        System::set_block_number(period);
        ModulePayments::<Test>::on_initialize(period);

        // After first block only two modules processed due to MaxPayoutsPerBlock = 2
        let cursor_after_first = crate::PendingPayoutCursor::<Test>::get();
        assert_eq!(cursor_after_first, Some(2));

        // Balances for first two owners should increase
        for account in [1u64, 2u64] {
            assert!(Balances::free_balance(&account) > balances_before[account as usize]);
        }
        for account in [3u64, 4u64] {
            assert_eq!(Balances::free_balance(&account), balances_before[account as usize]);
        }

        // Next block processes remaining queue regardless of distribution period alignment
        System::set_block_number(period + 1);
        ModulePayments::<Test>::on_initialize(period + 1);

        assert_eq!(crate::PendingPayoutCursor::<Test>::get(), None);
        for account in [3u64, 4u64] {
            assert!(Balances::free_balance(&account) > balances_before[account as usize]);
        }
    });
}
