use brickxinator_core::{
    inventory::{AssemblyState, Measure},
    ledger::{Body, Id, Ledger, Operation, ReceivedCopy, ReceivedLine, ReceivedStock},
    physical::*,
    verification,
};
fn op(body: Body) -> Operation {
    Operation {
        id: Id::new(),
        request_key: Id::new().to_string(),
        actor: "test".into(),
        reason: "reviewed physical event".into(),
        body,
    }
}
fn copy_fixture() -> (Ledger, Id, Id) {
    let mut l = Ledger::default();
    let copy = Id::new();
    let holding = Id::new();
    let line = Id::new();
    l.append(op(Body::Receive {
        purchase_id: Id::new(),
        lines: vec![ReceivedLine {
            invoice_line_id: Id::new(),
            stock: ReceivedStock::Sets {
                copies: vec![ReceivedCopy {
                    id: copy,
                    label: "Physical copy".into(),
                    assembly: AssemblyState::Assembled,
                }],
            },
        }],
    }))
    .unwrap();
    l.append(op(Body::Verify {
        copy_id: copy,
        expected_revision: 0,
        action: verification::Action::AddLine {
            line_id: line,
            holding_id: holding,
            definition_id: Some(Id::new()),
            color_id: Id::new(),
            expected: Some(100),
        },
    }))
    .unwrap();
    l.append(op(Body::Verify {
        copy_id: copy,
        expected_revision: 1,
        action: verification::Action::Count {
            line_id: line,
            total: 100,
        },
    }))
    .unwrap();
    (l, copy, holding)
}
#[test]
fn split_preserves_damage_and_independent_unknown_weight() {
    let (mut l, _copy, h) = copy_fixture();
    let child = Id::new();
    let rev = l.holdings()[&h].revision;
    l.append(op(Body::Split {
        source_id: h,
        child_id: child,
        expected_revision: rev,
        portion: Portion {
            pieces: Some(40),
            weight_mg: None,
            damaged_pieces: Some(0),
            damaged_weight_mg: None,
        },
    }))
    .unwrap();
    assert_eq!(l.holdings()[&h].pieces.owned(), Measure::Known(60));
    assert_eq!(l.holdings()[&child].pieces.owned(), Measure::Known(40));
    assert_eq!(l.holdings()[&child].weight_mg.owned(), Measure::Unknown);
    assert!(!l.set_contents().contains_key(&child));
    assert_eq!(l.holdings()[&child].assembly, AssemblyState::Loose);
}
#[test]
fn complete_sale_and_audited_inverse_preserve_copy_and_history() {
    let (mut l, copy, h) = copy_fixture();
    let event = op(Body::Lifecycle(Lifecycle {
        copy_id: copy,
        expected_revision: 0,
        status: Status::Sold,
        effective_on: "2026-11-23".into(),
        proceeds_cents: Some(45000),
        recipient: None,
        notes: String::new(),
        parts: vec![Disposition {
            holding_id: h,
            expected_revision: l.holdings()[&h].revision,
            handling: Handling::TransferAll,
        }],
        corrects: None,
    }));
    l.append(event.clone()).unwrap();
    assert_eq!(l.holdings()[&h].pieces.owned(), Measure::Known(0));
    assert_eq!(l.lifecycle(copy).status, Status::Sold);
    assert!(l.set_copies().contains_key(&copy));
    assert!(!l.set_contents().contains_key(&h));
    let correction = op(Body::Lifecycle(Lifecycle {
        copy_id: copy,
        expected_revision: 1,
        status: Status::Active,
        effective_on: "2026-11-23".into(),
        proceeds_cents: None,
        recipient: None,
        notes: "mistaken sale".into(),
        parts: vec![],
        corrects: Some(event.id),
    }));
    l.append(correction).unwrap();
    assert_eq!(l.holdings()[&h].pieces.owned(), Measure::Known(100));
    assert_eq!(l.set_contents()[&h], copy);
    assert_eq!(l.lifecycle(copy).status, Status::Active);
    let mut replay = Ledger::default();
    for o in l.operations() {
        replay.append(o.clone()).unwrap();
    }
    assert_eq!(replay.holdings(), l.holdings());
}
#[test]
fn custom_keep_is_explicit_and_later_use_blocks_inverse() {
    let (mut l, copy, h) = copy_fixture();
    let child = Id::new();
    let event = op(Body::Lifecycle(Lifecycle {
        copy_id: copy,
        expected_revision: 0,
        status: Status::Donated,
        effective_on: "2026-11-23".into(),
        proceeds_cents: None,
        recipient: Some("Recipient".into()),
        notes: String::new(),
        parts: vec![Disposition {
            holding_id: h,
            expected_revision: l.holdings()[&h].revision,
            handling: Handling::Custom {
                child_id: child,
                keep: Portion {
                    pieces: Some(10),
                    weight_mg: None,
                    damaged_pieces: Some(0),
                    damaged_weight_mg: None,
                },
            },
        }],
        corrects: None,
    }));
    l.append(event.clone()).unwrap();
    assert_eq!(l.holdings()[&child].pieces.owned(), Measure::Known(10));
    assert_eq!(l.holdings()[&h].pieces.owned(), Measure::Known(0));
    l.append(op(Body::Split {
        source_id: child,
        child_id: Id::new(),
        expected_revision: l.holdings()[&child].revision,
        portion: Portion {
            pieces: Some(1),
            weight_mg: None,
            damaged_pieces: Some(0),
            damaged_weight_mg: None,
        },
    }))
    .unwrap();
    let correction = op(Body::Lifecycle(Lifecycle {
        copy_id: copy,
        expected_revision: 1,
        status: Status::Active,
        effective_on: "2026-11-23".into(),
        proceeds_cents: None,
        recipient: None,
        notes: String::new(),
        parts: vec![],
        corrects: Some(event.id),
    }));
    assert!(l.append(correction).is_err());
    assert_eq!(l.lifecycle(copy).status, Status::Donated);
}
#[test]
fn missing_part_disposition_and_stale_source_are_atomic() {
    let (mut l, copy, h) = copy_fixture();
    let before = l.holdings().clone();
    let no_parts = op(Body::Lifecycle(Lifecycle {
        copy_id: copy,
        expected_revision: 0,
        status: Status::Lost,
        effective_on: "2026-11-23".into(),
        proceeds_cents: None,
        recipient: None,
        notes: String::new(),
        parts: vec![],
        corrects: None,
    }));
    assert!(l.append(no_parts).is_err());
    assert_eq!(l.holdings(), &before);
    assert_eq!(l.lifecycle(copy).status, Status::Active);
    assert!(
        l.append(op(Body::Detach {
            holding_id: h,
            expected_revision: 999
        }))
        .is_err()
    );
    assert_eq!(l.holdings(), &before);
}
