use crate::{ Error, Event, Pallet as ModulePayments, PaymentReport, mock::* };
use frame_support::{ assert_noop, assert_ok, BoundedVec };
extern crate alloc;
use pallet_modules::module::{ Module, ModuleTier };

fn bv(input: &[u8]) -> BoundedVec<u8, <Test as pallet_modules::Config>::MaxModuleNameLength> {
    BoundedVec::try_from(input.to_vec()).expect("within bound")
}

#[test]
fn set_authorized_module() {
    new_test_ext().execute_with(|| {
        let not_sudo: u64 = 1;
        assert_ok!(
            pallet_modules::Pallet::<Test>::register_module(
                RuntimeOrigin::signed(not_sudo),
                bv(b"authorized_module"),
                None,
                None,
                None
            )
        );

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
        assert_ok!(ModulePayments::<Test>::set_authorized_module(RuntimeOrigin::root(), 0u64));

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
        assert_ok!(ModulePayments::<Test>::set_authorized_module(RuntimeOrigin::root(), 0u64));

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
        assert_ok!(
            ModulePayments::<Test>::set_module_weights(
                RuntimeOrigin::signed(0u64),
                [1, 2].to_vec(),
                [80, 100].to_vec()
            )
        );

        let weight_values: Vec<u16> = crate::ModuleUsageWeights::<Test>
            ::iter_values()
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
        assert_ok!(ModulePayments::<Test>::set_authorized_module(RuntimeOrigin::root(), 0u64));

        // Reported payment fails if not authorized module
        assert_noop!(
            ModulePayments::<Test>::report_payment(RuntimeOrigin::signed(1u64), PaymentReport {
                module_id: 1u64,
                payee: 2u64,
                amount: 10_000_000_000_000,
            }),
            Error::<Test>::NotAuthorizedModule
        );

        // Reported payment fails if payment is nothing
        assert_noop!(
            ModulePayments::<Test>::report_payment(RuntimeOrigin::signed(0u64), PaymentReport {
                module_id: 1u64,
                payee: 2u64,
                amount: 0,
            }),
            Error::<Test>::EmptyPayment
        );

        // Reported payment fails if payee balance isn't enough
        assert_noop!(
            ModulePayments::<Test>::report_payment(RuntimeOrigin::signed(0u64), PaymentReport {
                module_id: 1u64,
                payee: 2u64,
                amount: 50_000_000_000_000,
            }),
            Error::<Test>::InsufficientFunds
        );

        // Reported payment fails if payee balance isn't enough to retain an existential balance
        assert_noop!(
            ModulePayments::<Test>::report_payment(RuntimeOrigin::signed(0u64), PaymentReport {
                module_id: 1u64,
                payee: 2u64,
                amount: 10_000_000_000_000,
            }),
            Error::<Test>::InsufficientFunds
        );

        // Succeeds otherwise
        assert_ok!(
            ModulePayments::<Test>::report_payment(RuntimeOrigin::signed(0u64), PaymentReport {
                module_id: 1u64,
                payee: 2u64,
                amount: 9_999_000_000_000,
            })
        );

        System::assert_last_event(
            (Event::ModulePaymentReported {
                module_id: 1u64,
                payee: 2u64,
                amount: 9_999_000_000_000,
                fee: 249_975_000_000,
            }).into()
        );
    })
}
