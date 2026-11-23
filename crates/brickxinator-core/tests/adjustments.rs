use brickxinator_core::{adjustments::*, finance::Method, ledger::Id, purchases::Purchase};
use std::collections::BTreeMap;
#[test]
fn financial_allocations_are_exact_stable_bounded_and_require_explicit_basis() {
    let a = Id::new();
    let b = Id::new();
    let purchase:Purchase=serde_json::from_value(serde_json::json!({"id":Id::new(),"revision":1,"status":"Ordered","archived":false,"draft":{"notes":"","lines":[{"id":a,"kind":"LooseBatch","pieces":{"Known":1},"weight_mg":{"Known":10},"paid_total_cents":100,"assembly":"Loose"},{"id":b,"kind":"LooseBatch","pieces":{"Known":1},"weight_mg":{"Known":10},"paid_total_cents":100,"assembly":"Loose"}]}})).unwrap();
    let entry = |method, kind, amount| Entry {
        id: Id::new(),
        kind,
        label: "Explicit adjustment".into(),
        effective_on: "2026-11-23".into(),
        amount_cents: amount,
        method,
        payment_method: None,
        warranty_id: None,
        return_id: None,
    };
    assert!(
        State::default()
            .apply(
                &Action::Record(entry(Method::Equal, Kind::Discount, 1)),
                &purchase
            )
            .is_err()
    );
    let state = State::default()
        .apply(
            &Action::SetGross {
                lines: BTreeMap::from([(a, 100), (b, 100)]),
            },
            &purchase,
        )
        .unwrap();
    for method in [
        Method::Equal,
        Method::PriceProportional,
        Method::PieceCount,
        Method::Weight,
    ] {
        let adjusted = state
            .apply(&Action::Record(entry(method, Kind::Coupon, 1)), &purchase)
            .unwrap();
        assert_eq!(adjusted.entries[0].shares[&a], 1);
        assert_eq!(adjusted.entries[0].shares[&b], 0);
        assert_eq!(adjusted.summary(&purchase).unwrap().net_lines[&a], Some(99));
    }
    let discounted = state
        .apply(
            &Action::Record(entry(Method::Equal, Kind::Coupon, 100)),
            &purchase,
        )
        .unwrap();
    assert!(
        discounted
            .apply(
                &Action::Record(entry(Method::Equal, Kind::Refund, 101)),
                &purchase
            )
            .is_err()
    );
    let last = discounted.entries[0].entry.id;
    let corrected = discounted
        .apply(&Action::CorrectLast { entry_id: last }, &purchase)
        .unwrap();
    assert_eq!(
        corrected.summary(&purchase).unwrap().net_lines[&a],
        Some(100)
    );
    assert_eq!(corrected.entries.len(), 1);
}
