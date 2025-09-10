use crate::{mock::*, Error, Event};
use frame_support::{assert_noop, assert_ok, BoundedVec};
extern crate alloc;
use alloc::vec;

#[test]
fn register_module_works() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited
        System::set_block_number(1);
    });
}
