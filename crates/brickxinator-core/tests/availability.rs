use Measure::{Known, Unknown};
use brickxinator_core::inventory::{AssemblyState, Balance, Measure};

#[test]
fn damage_reduces_availability_without_reducing_ownership() {
    let balance = Balance::new(Known(100), Known(2)).unwrap();
    assert_eq!(balance.owned(), Known(100));
    assert_eq!(balance.damaged(), Known(2));
    assert_eq!(balance.available(AssemblyState::Loose), Known(98));
}

#[test]
fn assembly_and_damage_overlap_without_double_subtraction() {
    let balance = Balance::new(Known(100), Known(2)).unwrap();
    for state in [AssemblyState::Assembled, AssemblyState::PartiallyAssembled] {
        assert_eq!(balance.available(state), Known(0));
        assert_eq!(balance.owned(), Known(100));
    }
    assert_eq!(balance.available(AssemblyState::Loose), Known(98));
}

#[test]
fn unknown_is_not_zero_and_measurements_remain_independent() {
    let pieces = Balance::new(Unknown, Unknown).unwrap();
    let weight = Balance::new(Known(5_000_000), Known(0)).unwrap();
    assert_eq!(pieces.available(AssemblyState::Loose), Unknown);
    assert_eq!(weight.available(AssemblyState::Loose), Known(5_000_000));
    let unknown_damage = Balance::new(Known(100), Unknown).unwrap();
    assert_eq!(unknown_damage.available(AssemblyState::Loose), Unknown);
}

#[test]
fn impossible_damage_is_rejected_and_empty_stock_has_no_availability() {
    assert!(Balance::new(Known(1), Known(2)).is_err());
    let empty = Balance::new(Known(0), Unknown).unwrap();
    assert_eq!(empty.available(AssemblyState::Loose), Known(0));
}
