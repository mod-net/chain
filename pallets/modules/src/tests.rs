use crate::{mock::*, Error, Event, Pallet as ModuleRegistry};
use frame_support::{assert_noop, assert_ok, BoundedVec};
extern crate alloc;
use alloc::vec;

fn bv(input: &[u8]) -> BoundedVec<u8, <Test as crate::Config>::MaxModuleNameLength> {
    BoundedVec::try_from(input.to_vec()).expect("within bound")
}

fn sr(input: &[u8]) -> crate::StorageReference<Test> {
    Some(BoundedVec::try_from(input.to_vec()).expect("within bound"))
}

#[test]
fn register_module_emits_event_and_updates_storage() {
    new_test_ext().execute_with(|| {
        let who: u64 = 1;
        let name = bv(b"mod-a");
        let data_ref = sr(b"ipfs://CID1");

        assert_ok!(ModuleRegistry::<Test>::register_module(RuntimeOrigin::signed(who), name.clone(), data_ref.clone(), None));

        // event: find the ModuleRegistered event among all runtime events
        let events = System::events();
        let ev = events.iter().find(|rec| matches!(rec.event, RuntimeEvent::ModuleRegistry(Event::ModuleRegistered{..})))
            .expect("ModuleRegistered event present");
        match &ev.event {
            RuntimeEvent::ModuleRegistry(Event::ModuleRegistered { who: e_who, id, name: e_name, data: e_data, collateral, take }) => {
                assert_eq!(*e_who, who);
                assert_eq!(*id, 0u64);
                assert_eq!(&e_name[..], &name[..]);
                assert_eq!(e_data.as_ref().map(|v| v.to_vec()), data_ref.as_ref().map(|v| v.to_vec()));
                assert_eq!(*collateral, DefaultModuleCollateral::get());
                assert_eq!(*take, sp_runtime::Percent::from_percent(0));
            }
            _ => unreachable!(),
        }

        // storage
        let m = crate::Modules::<Test>::get(0).expect("exists");
        assert_eq!(m.owner, who);
        assert_eq!(&m.name[..], &name[..]);
        assert_eq!(m.data.as_ref().map(|v| v.to_vec()), data_ref.as_ref().map(|v| v.to_vec()));
        assert_eq!(m.take, sp_runtime::Percent::from_percent(0));
        assert!(m.collateral > 0);
    });
}

#[test]
fn register_module_reserves_collateral_and_remove_unreserves() {
    new_test_ext().execute_with(|| {
        let who: u64 = 1;
        let name = bv(b"mod-a");
        let data_ref = sr(b"d");
        let before_free = Balances::free_balance(&who);
        let before_reserved = Balances::reserved_balance(&who);

        assert_ok!(ModuleRegistry::<Test>::register_module(RuntimeOrigin::signed(who), name.clone(), data_ref.clone(), None));

        let after_free = Balances::free_balance(&who);
        let after_reserved = Balances::reserved_balance(&who);
        assert_eq!(before_reserved + DefaultModuleCollateral::get(), after_reserved);
        assert_eq!(before_free - DefaultModuleCollateral::get(), after_free);

        assert_ok!(ModuleRegistry::<Test>::remove_module(RuntimeOrigin::signed(who), 0));
        assert_eq!(Balances::reserved_balance(&who), 0);
        assert_eq!(Balances::free_balance(&who), before_free);
    });
}

#[test]
fn register_module_respects_take_and_max_take() {
    new_test_ext().execute_with(|| {
        let who: u64 = 1;
        let name = bv(b"mod-a");
        let data_ref = sr(b"d");
        let ok_take = sp_runtime::Percent::from_percent(5);
        assert_ok!(ModuleRegistry::<Test>::register_module(RuntimeOrigin::signed(who), name.clone(), data_ref.clone(), Some(ok_take)));
        assert_eq!(crate::Modules::<Test>::get(0).unwrap().take, ok_take);

        // exceed
        let name2 = bv(b"mod-b");
        let too_high = sp_runtime::Percent::from_percent(DefaultMaxModuleTake::get().deconstruct() + 1);
        assert_noop!(ModuleRegistry::<Test>::register_module(RuntimeOrigin::signed(2), name2, sr(b"x"), Some(too_high)), Error::<Test>::MaxTakeExceeded);
    });
}

#[test]
fn register_module_name_validation() {
    new_test_ext().execute_with(|| {
        let who: u64 = 1;
        // empty
        assert_noop!(ModuleRegistry::<Test>::register_module(RuntimeOrigin::signed(who), bv(b""), sr(b"x"), None), Error::<Test>::NameEmpty);
        // leading space
        assert_noop!(ModuleRegistry::<Test>::register_module(RuntimeOrigin::signed(who), bv(b" foo"), sr(b"x"), None), Error::<Test>::NameWhitespace);
        // trailing space
        assert_noop!(ModuleRegistry::<Test>::register_module(RuntimeOrigin::signed(who), bv(b"foo "), sr(b"x"), None), Error::<Test>::NameWhitespace);
        // too long
        // length is enforced by BoundedVec type, so we skip testing NameLengthExceeded here.
        // not utf8 (invalid bytes)
        let invalid = vec![0xff, 0xfe, 0xfd];
        let name = BoundedVec::try_from(invalid).expect("raw bytes allowed, checked inside");
        assert_noop!(ModuleRegistry::<Test>::register_module(RuntimeOrigin::signed(who), name, sr(b"x"), None), Error::<Test>::NameNotUTF8);
        // name taken
        assert_ok!(ModuleRegistry::<Test>::register_module(RuntimeOrigin::signed(who), bv(b"dup"), sr(b"x"), None));
        assert_noop!(ModuleRegistry::<Test>::register_module(RuntimeOrigin::signed(2), bv(b"dup"), sr(b"y"), None), Error::<Test>::NameTaken);
    });
}

#[test]
fn update_module_works_and_checks_ownership_and_take() {
    new_test_ext().execute_with(|| {
        let who: u64 = 1;
        assert_ok!(ModuleRegistry::<Test>::register_module(RuntimeOrigin::signed(who), bv(b"a"), sr(b"x"), None));
        // update name only
        assert_ok!(ModuleRegistry::<Test>::update_module(RuntimeOrigin::signed(who), 0, Some(bv(b"b")), None, None));
        let m = crate::Modules::<Test>::get(0).unwrap();
        assert_eq!(&m.name[..], b"b");
        // update data and take
        let new_take = sp_runtime::Percent::from_percent(3);
        assert_ok!(ModuleRegistry::<Test>::update_module(RuntimeOrigin::signed(who), 0, None, sr(b"y"), Some(new_take)));
        let m = crate::Modules::<Test>::get(0).unwrap();
        assert_eq!(m.data.as_ref().unwrap().to_vec(), b"y".to_vec());
        assert_eq!(m.take, new_take);
        // max take exceed
        let too = sp_runtime::Percent::from_percent(DefaultMaxModuleTake::get().deconstruct() + 1);
        assert_noop!(ModuleRegistry::<Test>::update_module(RuntimeOrigin::signed(who), 0, None, None, Some(too)), Error::<Test>::MaxTakeExceeded);
        // ownership
        assert_noop!(ModuleRegistry::<Test>::update_module(RuntimeOrigin::signed(2), 0, Some(bv(b"c")), None, None), Error::<Test>::ModuleOwnership);
        // not found
        assert_noop!(ModuleRegistry::<Test>::update_module(RuntimeOrigin::signed(who), 999, None, None, None), Error::<Test>::ModuleNotFound);
    });
}

#[test]
fn remove_module_checks_ownership_and_existence() {
    new_test_ext().execute_with(|| {
        let who: u64 = 1;
        assert_ok!(ModuleRegistry::<Test>::register_module(RuntimeOrigin::signed(who), bv(b"a"), sr(b"x"), None));
        assert_noop!(ModuleRegistry::<Test>::remove_module(RuntimeOrigin::signed(2), 0), Error::<Test>::ModuleOwnership);
        assert_ok!(ModuleRegistry::<Test>::remove_module(RuntimeOrigin::signed(who), 0));
        assert!(crate::Modules::<Test>::get(0).is_none());
        assert_noop!(ModuleRegistry::<Test>::remove_module(RuntimeOrigin::signed(who), 0), Error::<Test>::ModuleNotFound);
    });
}

#[test]
fn max_modules_reached() {
    new_test_ext().execute_with(|| {
        // MaxModules set to 3 in mock. We allow ids 0,1 then the check rejects when next id plus 1 equals limit.
        assert_ok!(ModuleRegistry::<Test>::register_module(RuntimeOrigin::signed(1), bv(b"a"), sr(b"x"), None));
        assert_ok!(ModuleRegistry::<Test>::register_module(RuntimeOrigin::signed(2), bv(b"b"), sr(b"y"), None));
        // Next is 2, saturating_add(1) == 3 which equals MaxModules -> error
        assert_noop!(ModuleRegistry::<Test>::register_module(RuntimeOrigin::signed(3), bv(b"c"), sr(b"z"), None), Error::<Test>::MaxModulesReached);
    });
}
