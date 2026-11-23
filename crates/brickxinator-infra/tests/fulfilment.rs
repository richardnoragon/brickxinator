use brickxinator_core::{
    fulfilment::*,
    inventory::Measure,
    ledger::{Id, ReceivedLine, ReceivedStock},
    purchases::{Draft, PurchaseAction, PurchaseCommand},
};
use brickxinator_infra::{exchange, sqlite::Store};
fn setup() -> (tempfile::TempDir, Store, Id, Id) {
    let root = tempfile::tempdir().unwrap();
    let mut s = Store::create(root.path().join("collection.sqlite")).unwrap();
    let p = Id::new();
    let l = Id::new();
    let draft:Draft=serde_json::from_value(serde_json::json!({"notes":"","lines":[{"id":l,"kind":"LooseBatch","description":"Ordered bricks","pieces":{"Known":100},"weight_mg":"Unknown","paid_total_cents":1000,"assembly":"Loose"}]})).unwrap();
    s.execute_purchase(PurchaseCommand {
        id: Id::new(),
        request_key: Id::new().to_string(),
        actor: "test".into(),
        reason: "order".into(),
        body: PurchaseAction::Create {
            purchase_id: p,
            draft,
        },
    })
    .unwrap();
    (root, s, p, l)
}
fn command(s: &Store, p: Id, action: Action) -> Command {
    let v = s.fulfilment(p).unwrap();
    Command {
        id: Id::new(),
        request_key: Id::new().to_string(),
        actor: "test".into(),
        reason: "reviewed receipt".into(),
        purchase_id: p,
        expected_revision: v["state"]["revision"].as_u64().unwrap(),
        expected_purchase_revision: v["purchase"]["revision"].as_u64().unwrap(),
        action,
    }
}
fn apply(s: &mut Store, c: Command) -> State {
    let v = s.preview_fulfilment(c.clone()).unwrap();
    s.execute_fulfilment(
        c,
        v["expected_inventory_sequence"].as_u64().unwrap() as usize,
        v["expected_profile_sequence"].as_i64().unwrap(),
    )
    .unwrap()
}
fn receipt(l: Id, n: u64) -> Action {
    Action::Receipt(Receipt {
        id: Id::new(),
        shipment_id: None,
        purpose: Purpose::Purchase,
        effective_on: "2026-11-23".into(),
        lines: vec![ReceivedLine {
            invoice_line_id: l,
            stock: ReceivedStock::Batch {
                holding_id: Id::new(),
                pieces: Measure::Known(n),
                weight_mg: Measure::Unknown,
            },
        }],
        notes: "actual delivery".into(),
    })
}
#[test]
fn partial_receipts_backorders_exact_totals_and_native_restore() {
    let (root, mut s, p, l) = setup();
    let first = command(&s, p, receipt(l, 80));
    let result = apply(&mut s, first.clone());
    assert_eq!(s.execute_fulfilment(first, 0, 0).unwrap(), result);
    assert_eq!(s.fulfilment(p).unwrap()["status"], "PartiallyReceived");
    let c = command(
        &s,
        p,
        Action::Backorder(Backorder {
            line_id: l,
            pieces: Some(20),
            weight_mg: None,
            copies: None,
            notes: "supplier delay".into(),
        }),
    );
    apply(&mut s, c);
    assert_eq!(s.fulfilment(p).unwrap()["status"], "Backordered");
    assert!(
        s.preview_fulfilment(command(&s, p, receipt(l, 21)))
            .is_err()
    );
    let c = command(&s, p, receipt(l, 20));
    apply(&mut s, c);
    assert_eq!(s.fulfilment(p).unwrap()["status"], "Completed");
    assert_eq!(
        s.dashboard().unwrap().inventory.all.owned_pieces.known_sum,
        100
    );
    assert!(s.preview_fulfilment(command(&s, p, receipt(l, 1))).is_err());
    drop(s);
    let source = root.path().join("collection.sqlite");
    let view = Store::open(&source).unwrap().fulfilment(p).unwrap();
    let native = root.path().join("native");
    exchange::export(&source, &native).unwrap();
    let target = root.path().join("restored.sqlite");
    drop(Store::create(&target).unwrap());
    exchange::import(&target, &native, root.path().join("safe.zip")).unwrap();
    assert_eq!(Store::open(&target).unwrap().fulfilment(p).unwrap(), view);
}
#[test]
fn receipt_reversal_and_correction_are_audited_without_deletion() {
    let (root, mut s, p, l) = setup();
    let c = command(&s, p, receipt(l, 80));
    let receipt_op = c.id;
    apply(&mut s, c);
    let c = command(
        &s,
        p,
        Action::ReceiptCorrection {
            receipt_operation_id: receipt_op,
            reverse: true,
        },
    );
    apply(&mut s, c);
    assert_eq!(
        s.dashboard().unwrap().inventory.all.owned_pieces.known_sum,
        0
    );
    assert_eq!(s.fulfilment(p).unwrap()["status"], "Open");
    let c = command(
        &s,
        p,
        Action::ReceiptCorrection {
            receipt_operation_id: receipt_op,
            reverse: false,
        },
    );
    apply(&mut s, c);
    assert_eq!(
        s.dashboard().unwrap().inventory.all.owned_pieces.known_sum,
        80
    );
    assert_eq!(s.rebuild().unwrap().operations().len(), 3);
    drop(s);
    Store::open(root.path().join("collection.sqlite")).unwrap();
}
#[test]
fn warranty_status_is_not_stock_and_closed_orders_allow_actual_replacement() {
    let (_root, mut s, p, l) = setup();
    let claim = Id::new();
    let c = command(
        &s,
        p,
        Action::Warranty(Warranty {
            id: claim,
            label: "Replacement".into(),
            status: WarrantyStatus::ReplacementSent,
            notes: "".into(),
        }),
    );
    apply(&mut s, c);
    assert_eq!(
        s.dashboard().unwrap().inventory.all.owned_pieces.known_sum,
        0
    );
    let c = command(&s, p, Action::Close { closed: true });
    apply(&mut s, c);
    let Action::Receipt(mut r) = receipt(l, 120) else {
        unreachable!()
    };
    r.purpose = Purpose::Warranty { claim_id: claim };
    let c = command(&s, p, Action::Receipt(r));
    apply(&mut s, c);
    assert_eq!(
        s.dashboard().unwrap().inventory.all.owned_pieces.known_sum,
        120
    );
    assert_eq!(
        s.fulfilment(p).unwrap()["coverage"][0]["received"],
        serde_json::json!({"Known":0})
    );
}
#[test]
fn changed_receipt_stock_blocks_reversal_without_losing_history() {
    use brickxinator_core::{ledger::Body, physical::Portion};
    let (_root, mut s, p, l) = setup();
    let action = receipt(l, 80);
    let Action::Receipt(r) = &action else {
        unreachable!()
    };
    let ReceivedStock::Batch { holding_id, .. } = r.lines[0].stock else {
        unreachable!()
    };
    let c = command(&s, p, action);
    let receipt_id = c.id;
    apply(&mut s, c);
    let operation = brickxinator_core::ledger::Operation {
        id: Id::new(),
        request_key: Id::new().to_string(),
        actor: "test".into(),
        reason: "later physical split".into(),
        body: Body::Split {
            source_id: holding_id,
            child_id: Id::new(),
            expected_revision: s.rebuild().unwrap().holdings()[&holding_id].revision,
            portion: Portion {
                pieces: Some(10),
                weight_mg: None,
                damaged_pieces: Some(0),
                damaged_weight_mg: None,
            },
        },
    };
    let preview = s.preview_physical(operation.clone()).unwrap();
    s.execute_physical(
        operation,
        preview["expected_profile_sequence"].as_i64().unwrap(),
    )
    .unwrap();
    let inverse = command(
        &s,
        p,
        Action::ReceiptCorrection {
            receipt_operation_id: receipt_id,
            reverse: true,
        },
    );
    assert!(s.preview_fulfilment(inverse).is_err());
    assert_eq!(
        s.dashboard().unwrap().inventory.all.owned_pieces.known_sum,
        80
    );
    assert!(!s.rebuild().unwrap().receipt_reversed(receipt_id));
}

#[test]
fn discounts_refunds_and_gift_cards_are_exact_and_never_change_stock() {
    use brickxinator_core::{
        adjustments::{Action as F, Entry, Kind, PaymentMethod},
        finance::Method,
    };
    let (_root, mut s, p, l) = setup();
    let c = command(
        &s,
        p,
        Action::Financial(F::SetGross {
            lines: std::collections::BTreeMap::from([(l, 50000)]),
        }),
    );
    apply(&mut s, c);
    for (kind, amount, instrument) in [
        (Kind::Coupon, 2500, None),
        (Kind::LoyaltyReduction, 1000, None),
        (Kind::Payment, 10000, Some(PaymentMethod::GiftCard)),
        (Kind::Refund, 1000, None),
    ] {
        let c = command(
            &s,
            p,
            Action::Financial(F::Record(Entry {
                id: Id::new(),
                kind,
                label: format!("{kind:?}"),
                effective_on: "2026-11-23".into(),
                amount_cents: amount,
                method: Method::PriceProportional,
                payment_method: instrument,
                warranty_id: None,
                return_id: None,
            })),
        );
        apply(&mut s, c);
    }
    let view = s.fulfilment(p).unwrap();
    assert_eq!(view["financial_summary"]["net_lines"][l.to_string()], 46500);
    assert_eq!(
        view["financial_summary"]["refunded_lines"][l.to_string()],
        1000
    );
    assert_eq!(view["financial_summary"]["payments_cents"], 10000);
    assert_eq!(
        view["purchase"]["draft"]["lines"][0]["paid_total_cents"],
        1000
    );
    assert_eq!(s.rebuild().unwrap().operations().len(), 0);
}
#[test]
fn planned_dispatch_acknowledgment_and_correction_return_stock_once() {
    use brickxinator_core::{
        physical::Portion,
        supplier_returns::{Action as R, Return, Target},
    };
    let (root, mut s, p, l) = setup();
    let action = receipt(l, 80);
    let Action::Receipt(r) = &action else {
        unreachable!()
    };
    let ReceivedStock::Batch { holding_id, .. } = r.lines[0].stock else {
        unreachable!()
    };
    let c = command(&s, p, action);
    apply(&mut s, c);
    let id = Id::new();
    let c = command(
        &s,
        p,
        Action::SupplierReturn(R::Plan(Return {
            id,
            label: "Wrong parts".into(),
            requested_on: "2026-11-23".into(),
            notes: "".into(),
            warranty_id: None,
            dispatched_by: None,
            acknowledged_on: None,
            corrected_by: None,
        })),
    );
    apply(&mut s, c);
    assert_eq!(
        s.dashboard().unwrap().inventory.all.owned_pieces.known_sum,
        80
    );
    let c = command(
        &s,
        p,
        Action::SupplierReturn(R::Dispatch {
            return_id: id,
            effective_on: "2026-11-23".into(),
            target: Target::Holding {
                holding_id,
                sources: vec![],
                expected_revision: s.rebuild().unwrap().holdings()[&holding_id].revision,
                portion: Portion {
                    pieces: Some(20),
                    weight_mg: None,
                    damaged_pieces: Some(0),
                    damaged_weight_mg: None,
                },
            },
        }),
    );
    let dispatch = c.id;
    apply(&mut s, c);
    assert_eq!(
        s.dashboard().unwrap().inventory.all.owned_pieces.known_sum,
        60
    );
    let c = command(
        &s,
        p,
        Action::SupplierReturn(R::Acknowledge {
            return_id: id,
            effective_on: "2026-11-23".into(),
        }),
    );
    apply(&mut s, c);
    assert_eq!(
        s.dashboard().unwrap().inventory.all.owned_pieces.known_sum,
        60
    );
    let c = command(
        &s,
        p,
        Action::SupplierReturn(R::CorrectDispatch {
            return_id: id,
            dispatch_operation_id: dispatch,
            effective_on: "2026-11-23".into(),
        }),
    );
    apply(&mut s, c);
    assert_eq!(
        s.dashboard().unwrap().inventory.all.owned_pieces.known_sum,
        80
    );
    drop(s);
    Store::open(root.path().join("collection.sqlite")).unwrap();
}
#[test]
fn returned_copy_and_warranty_replacement_keep_distinct_permanent_identities() {
    use brickxinator_core::{
        inventory::AssemblyState,
        ledger::ReceivedCopy,
        physical::{Lifecycle, Status},
        supplier_returns::{Action as R, Return, Target},
    };
    let (root, mut s, p, l) = setup();
    let mut purchase = s.purchases().unwrap().remove(0);
    purchase.draft.lines[0].kind = brickxinator_core::purchases::LineKind::Set;
    purchase.draft.lines[0].copy_count = Some(1);
    s.execute_purchase(PurchaseCommand {
        id: Id::new(),
        request_key: Id::new().to_string(),
        actor: "test".into(),
        reason: "set order".into(),
        body: PurchaseAction::Save {
            purchase_id: p,
            expected_revision: 1,
            draft: purchase.draft,
            status: brickxinator_core::purchases::Status::Ordered,
        },
    })
    .unwrap();
    let original = Id::new();
    let c = command(
        &s,
        p,
        Action::Receipt(Receipt {
            id: Id::new(),
            shipment_id: None,
            purpose: Purpose::Purchase,
            effective_on: "2026-11-23".into(),
            notes: "original".into(),
            lines: vec![ReceivedLine {
                invoice_line_id: l,
                stock: ReceivedStock::Sets {
                    copies: vec![ReceivedCopy {
                        id: original,
                        label: "Original set".into(),
                        assembly: AssemblyState::Loose,
                    }],
                },
            }],
        }),
    );
    apply(&mut s, c);
    let ret = Id::new();
    let c = command(
        &s,
        p,
        Action::SupplierReturn(R::Plan(Return {
            id: ret,
            label: "Faulty set".into(),
            requested_on: "2026-11-23".into(),
            notes: "".into(),
            warranty_id: None,
            dispatched_by: None,
            acknowledged_on: None,
            corrected_by: None,
        })),
    );
    apply(&mut s, c);
    let c = command(
        &s,
        p,
        Action::SupplierReturn(R::Dispatch {
            return_id: ret,
            effective_on: "2026-11-23".into(),
            target: Target::Copy(Lifecycle {
                copy_id: original,
                expected_revision: 0,
                status: Status::Returned,
                effective_on: "2026-11-23".into(),
                proceeds_cents: None,
                recipient: None,
                notes: "actual handover".into(),
                parts: vec![],
                corrects: None,
            }),
        }),
    );
    apply(&mut s, c);
    assert_eq!(
        s.rebuild().unwrap().lifecycle(original).status,
        Status::Returned
    );
    let claim = Id::new();
    let c = command(
        &s,
        p,
        Action::Warranty(Warranty {
            id: claim,
            label: "Replacement".into(),
            status: WarrantyStatus::Approved,
            notes: "".into(),
        }),
    );
    apply(&mut s, c);
    let replacement = Id::new();
    let c = command(
        &s,
        p,
        Action::Receipt(Receipt {
            id: Id::new(),
            shipment_id: None,
            purpose: Purpose::Warranty { claim_id: claim },
            effective_on: "2026-11-23".into(),
            notes: "different physical object".into(),
            lines: vec![ReceivedLine {
                invoice_line_id: l,
                stock: ReceivedStock::Sets {
                    copies: vec![ReceivedCopy {
                        id: replacement,
                        label: "Replacement set".into(),
                        assembly: AssemblyState::Loose,
                    }],
                },
            }],
        }),
    );
    apply(&mut s, c);
    let ledger = s.rebuild().unwrap();
    assert_eq!(ledger.lifecycle(original).status, Status::Returned);
    assert_eq!(ledger.lifecycle(replacement).status, Status::Active);
    assert_eq!(ledger.set_copies().len(), 2);
    drop(s);
    Store::open(root.path().join("collection.sqlite")).unwrap();
}
#[test]
fn direct_return_correction_revalidates_metadata_even_without_a_preview() {
    use brickxinator_core::{
        physical::Portion,
        profiles,
        supplier_returns::{Action as R, Return, Target},
    };
    let (_root, mut s, p, l) = setup();
    let action = receipt(l, 80);
    let Action::Receipt(r) = &action else {
        unreachable!()
    };
    let ReceivedStock::Batch { holding_id, .. } = r.lines[0].stock else {
        unreachable!()
    };
    let c = command(&s, p, action);
    apply(&mut s, c);
    let id = Id::new();
    let c = command(
        &s,
        p,
        Action::SupplierReturn(R::Plan(Return {
            id,
            label: "Returned pieces".into(),
            requested_on: "2026-11-23".into(),
            notes: "".into(),
            warranty_id: None,
            dispatched_by: None,
            acknowledged_on: None,
            corrected_by: None,
        })),
    );
    apply(&mut s, c);
    let c = command(
        &s,
        p,
        Action::SupplierReturn(R::Dispatch {
            return_id: id,
            effective_on: "2026-11-23".into(),
            target: Target::Holding {
                holding_id,
                expected_revision: s.rebuild().unwrap().holdings()[&holding_id].revision,
                portion: Portion {
                    pieces: Some(20),
                    weight_mg: None,
                    damaged_pieces: Some(0),
                    damaged_weight_mg: None,
                },
                sources: vec![],
            },
        }),
    );
    let dispatch = c.id;
    apply(&mut s, c);
    let mut links = s.default_inventory_links().unwrap();
    links.archived = true;
    s.execute_profile(profiles::Command {
        id: Id::new(),
        request_key: Id::new().to_string(),
        actor: "test".into(),
        reason: "later independent profile change".into(),
        target: profiles::Target::Holding(holding_id),
        expected_revision: 0,
        links,
    })
    .unwrap();
    let c = command(
        &s,
        p,
        Action::SupplierReturn(R::CorrectDispatch {
            return_id: id,
            dispatch_operation_id: dispatch,
            effective_on: "2026-11-23".into(),
        }),
    );
    let view = s.fulfilment(p).unwrap();
    assert!(
        s.execute_fulfilment(
            c,
            view["inventory_sequence"].as_u64().unwrap() as usize,
            view["profile_sequence"].as_i64().unwrap()
        )
        .is_err()
    );
    assert_eq!(
        s.rebuild().unwrap().holdings()[&holding_id].pieces.owned(),
        Measure::Known(60)
    );
}
