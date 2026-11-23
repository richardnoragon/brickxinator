use brickxinator_core::{
    child_costs::{AllocationMethod, Command, Component},
    inventory::Measure,
    ledger::{Body, Id, Operation},
    physical::Portion,
    purchases::{Draft, PurchaseAction, PurchaseCommand},
};
use brickxinator_infra::{exchange, sqlite::Store};
use std::collections::BTreeMap;
#[test]
fn costs_follow_splits_without_changing_original_prices_and_survive_restore() {
    let root = tempfile::tempdir().unwrap();
    let db = root.path().join("collection.sqlite");
    let mut s = Store::create(&db).unwrap();
    let purchase = Id::new();
    let line = Id::new();
    let draft:Draft=serde_json::from_value(serde_json::json!({"shipping_cents":0,"fees_cents":0,"notes":"","lines":[{"id":line,"kind":"MixedLot","description":"100 pieces","pieces":{"Known":100},"weight_mg":"Unknown","volume_ml":1000,"paid_total_cents":2500,"assembly":"Loose"}]})).unwrap();
    let c = |body| PurchaseCommand {
        id: Id::new(),
        request_key: Id::new().to_string(),
        actor: "test".into(),
        reason: "actual acquisition".into(),
        body,
    };
    s.execute_purchase(c(PurchaseAction::Create {
        purchase_id: purchase,
        draft,
    }))
    .unwrap();
    s.execute_purchase(c(PurchaseAction::Receive {
        purchase_id: purchase,
        expected_revision: 1,
    }))
    .unwrap();
    let ledger = s.rebuild().unwrap();
    let holding = *ledger.holdings().keys().next().unwrap();
    let view = s.child_costs(purchase).unwrap();
    let command = Command {
        id: Id::new(),
        request_key: Id::new().to_string(),
        actor: "test".into(),
        reason: "initial known cost layer".into(),
        effective_on: "2026-11-23".into(),
        purchase_id: purchase,
        invoice_line_id: Some(line),
        component: Component::Item,
        expected_revision: 0,
        expected_inventory_sequence: ledger.operations().len(),
        expected_purchase_revision: 2,
        expected_fulfilment_revision: 0,
        expected_shipping_revision: 0,
        method: AllocationMethod::Manual,
        reviewed_disposed_cents: None,
        copy_targets: BTreeMap::new(),
        copy_piece_bases: BTreeMap::new(),
        allocated_cents: 2500,
        targets: BTreeMap::from([(holding, Some(2500))]),
    };
    assert_eq!(view["plans"].as_array().unwrap().len(), 0);
    let plan = s.preview_child_cost(command.clone()).unwrap();
    assert_eq!(plan.total_cents, 2500);
    s.execute_child_cost(command.clone()).unwrap();
    assert_eq!(s.execute_child_cost(command).unwrap(), plan);
    let child = Id::new();
    let op = Operation {
        id: Id::new(),
        request_key: Id::new().to_string(),
        actor: "test".into(),
        reason: "separate 40 pieces".into(),
        body: Body::Split {
            source_id: holding,
            child_id: child,
            expected_revision: ledger.holdings()[&holding].revision,
            portion: Portion {
                pieces: Some(40),
                weight_mg: None,
                damaged_pieces: Some(0),
                damaged_weight_mg: None,
            },
        },
    };
    let preview = s.preview_physical(op.clone()).unwrap();
    s.execute_physical(op, preview["expected_profile_sequence"].as_i64().unwrap())
        .unwrap();
    let view = s.child_costs(purchase).unwrap();
    assert_eq!(
        view["plans"][0]["current"]["shares"][holding.to_string()],
        1500
    );
    assert_eq!(
        view["plans"][0]["current"]["shares"][child.to_string()],
        1000
    );
    assert_eq!(
        s.purchases().unwrap()[0].draft.lines[0].paid_total_cents,
        Some(2500)
    );
    assert_eq!(
        s.rebuild().unwrap().holdings()[&holding].pieces.owned(),
        Measure::Known(60)
    );
    drop(s);
    let native = root.path().join("native");
    exchange::export(&db, &native).unwrap();
    let target = root.path().join("restored.sqlite");
    drop(Store::create(&target).unwrap());
    exchange::import(&target, &native, root.path().join("safe.zip")).unwrap();
    assert_eq!(
        Store::open(&target).unwrap().child_costs(purchase).unwrap(),
        view
    );
}
#[test]
fn explicit_merged_return_consumes_only_its_selected_acquisition_cost_layer() {
    use brickxinator_core::{
        catalog,
        fulfilment::{self, Action as F},
        physical::{AcquisitionPortion, Source},
        profiles,
        settings::{Kind, Vocabulary},
        supplier_returns::{self, Target},
    };
    let root = tempfile::tempdir().unwrap();
    let mut s = Store::create(root.path().join("db.sqlite")).unwrap();
    let manufacturer = s
        .settings()
        .unwrap()
        .values()
        .find(|r| {
            matches!(
                r.kind,
                Kind::Vocabulary {
                    vocabulary: Vocabulary::Manufacturer,
                    system_unknown: true
                }
            )
        })
        .unwrap()
        .id;
    let part = Id::new();
    s.execute_catalog(catalog::Command {
        id: Id::new(),
        request_key: Id::new().to_string(),
        actor: "test".into(),
        reason: "compatible part identity".into(),
        action: catalog::Action::Create {
            id: part,
            kind: catalog::Kind::Part,
            details: serde_json::from_value(
                serde_json::json!({"manufacturer_id":manufacturer,"number":"3001","name":"Brick"}),
            )
            .unwrap(),
        },
    })
    .unwrap();
    let mut sources = vec![];
    for price in [1000, 2000] {
        let p = Id::new();
        let l = Id::new();
        let draft:Draft=serde_json::from_value(serde_json::json!({"shipping_cents":0,"fees_cents":0,"notes":"","lines":[{"id":l,"kind":"LooseBatch","pieces":{"Known":100},"weight_mg":"Unknown","volume_ml":1000,"paid_total_cents":price,"assembly":"Loose"}]})).unwrap();
        let c = |body| PurchaseCommand {
            id: Id::new(),
            request_key: Id::new().to_string(),
            actor: "test".into(),
            reason: "real acquisition".into(),
            body,
        };
        s.execute_purchase(c(PurchaseAction::Create {
            purchase_id: p,
            draft,
        }))
        .unwrap();
        let receipt = s
            .execute_purchase(c(PurchaseAction::Receive {
                purchase_id: p,
                expected_revision: 1,
            }))
            .unwrap();
        let h = *receipt.receipt.unwrap().holdings.keys().next().unwrap();
        let mut links = s.default_inventory_links().unwrap();
        links.definition_id = Some(part);
        s.execute_profile(profiles::Command {
            id: Id::new(),
            request_key: Id::new().to_string(),
            actor: "test".into(),
            reason: "part identification".into(),
            target: profiles::Target::Holding(h),
            expected_revision: 0,
            links,
        })
        .unwrap();
        let command = Command {
            id: Id::new(),
            request_key: Id::new().to_string(),
            actor: "test".into(),
            reason: "explicit source cost".into(),
            effective_on: "2026-11-23".into(),
            purchase_id: p,
            invoice_line_id: Some(l),
            component: Component::Item,
            expected_revision: 0,
            expected_inventory_sequence: s.rebuild().unwrap().operations().len(),
            expected_purchase_revision: 2,
            expected_fulfilment_revision: 0,
            expected_shipping_revision: 0,
            method: AllocationMethod::Manual,
            reviewed_disposed_cents: None,
            copy_targets: BTreeMap::new(),
            copy_piece_bases: BTreeMap::new(),
            allocated_cents: price,
            targets: BTreeMap::from([(h, Some(price))]),
        };
        s.execute_child_cost(command).unwrap();
        sources.push((p, l, h));
    }
    let merged = Id::new();
    let ledger = s.rebuild().unwrap();
    let op = Operation {
        id: Id::new(),
        request_key: Id::new().to_string(),
        actor: "test".into(),
        reason: "merge compatible parts with provenance".into(),
        body: Body::Merge {
            sources: sources
                .iter()
                .map(|(_, _, h)| Source {
                    holding_id: *h,
                    expected_revision: ledger.holdings()[h].revision,
                })
                .collect(),
            target_id: merged,
        },
    };
    let preview = s.preview_physical(op.clone()).unwrap();
    s.execute_physical(op, preview["expected_profile_sequence"].as_i64().unwrap())
        .unwrap();
    let (p, line, _) = sources[0];
    let id = Id::new();
    let action = F::SupplierReturn(supplier_returns::Action::Plan(supplier_returns::Return {
        id,
        label: "Supplier return".into(),
        requested_on: "2026-11-23".into(),
        notes: "".into(),
        warranty_id: None,
        dispatched_by: None,
        acknowledged_on: None,
        corrected_by: None,
    }));
    let command = |revision, action| fulfilment::Command {
        id: Id::new(),
        request_key: Id::new().to_string(),
        actor: "test".into(),
        reason: "explicit source return".into(),
        purchase_id: p,
        expected_revision: revision,
        expected_purchase_revision: 2,
        action,
    };
    let c = command(0, action);
    let v = s.preview_fulfilment(c.clone()).unwrap();
    s.execute_fulfilment(
        c,
        v["expected_inventory_sequence"].as_u64().unwrap() as usize,
        v["expected_profile_sequence"].as_i64().unwrap(),
    )
    .unwrap();
    let c = command(
        1,
        F::SupplierReturn(supplier_returns::Action::Dispatch {
            return_id: id,
            effective_on: "2026-11-23".into(),
            target: Target::Holding {
                holding_id: merged,
                expected_revision: s.rebuild().unwrap().holdings()[&merged].revision,
                portion: Portion {
                    pieces: Some(20),
                    weight_mg: None,
                    damaged_pieces: Some(0),
                    damaged_weight_mg: None,
                },
                sources: vec![AcquisitionPortion {
                    invoice_line_id: line,
                    pieces: Some(20),
                    weight_mg: None,
                }],
            },
        }),
    );
    let v = s.preview_fulfilment(c.clone()).unwrap();
    s.execute_fulfilment(
        c,
        v["expected_inventory_sequence"].as_u64().unwrap() as usize,
        v["expected_profile_sequence"].as_i64().unwrap(),
    )
    .unwrap();
    let a = s.child_costs(p).unwrap();
    let b = s.child_costs(sources[1].0).unwrap();
    assert_eq!(a["plans"][0]["current"]["shares"][merged.to_string()], 800);
    assert_eq!(a["plans"][0]["current"]["disposed_cents"], 200);
    assert_eq!(b["plans"][0]["current"]["shares"][merged.to_string()], 2000);
    assert_eq!(b["plans"][0]["current"]["disposed_cents"], 0);
    let refund = fulfilment::Command {
        id: Id::new(),
        request_key: Id::new().to_string(),
        actor: "test".into(),
        reason: "supplier full refund independently from stock".into(),
        purchase_id: p,
        expected_revision: 2,
        expected_purchase_revision: 2,
        action: F::Financial(brickxinator_core::adjustments::Action::Record(
            brickxinator_core::adjustments::Entry {
                id: Id::new(),
                kind: brickxinator_core::adjustments::Kind::Refund,
                label: "Full refund".into(),
                effective_on: "2026-11-23".into(),
                amount_cents: 1000,
                method: brickxinator_core::finance::Method::PriceProportional,
                payment_method: None,
                warranty_id: None,
                return_id: Some(id),
            },
        )),
    };
    let v = s.preview_fulfilment(refund.clone()).unwrap();
    s.execute_fulfilment(
        refund,
        v["expected_inventory_sequence"].as_u64().unwrap() as usize,
        v["expected_profile_sequence"].as_i64().unwrap(),
    )
    .unwrap();
    assert_eq!(
        s.child_costs(p).unwrap()["plans"][0]["requires_reallocation"],
        true
    );
    let mut correction = Command {
        id: Id::new(),
        request_key: Id::new().to_string(),
        actor: "test".into(),
        reason: "review remaining and disposed source costs after full refund".into(),
        effective_on: "2026-11-23".into(),
        purchase_id: p,
        invoice_line_id: Some(line),
        component: Component::Item,
        expected_revision: 1,
        expected_inventory_sequence: s.rebuild().unwrap().operations().len(),
        expected_purchase_revision: 2,
        expected_fulfilment_revision: 3,
        expected_shipping_revision: 0,
        method: AllocationMethod::Manual,
        copy_targets: BTreeMap::new(),
        copy_piece_bases: BTreeMap::new(),
        allocated_cents: 0,
        reviewed_disposed_cents: None,
        targets: BTreeMap::from([(merged, Some(0))]),
    };
    assert!(s.preview_child_cost(correction.clone()).is_err());
    correction.reviewed_disposed_cents = Some(0);
    s.execute_child_cost(correction).unwrap();
    let cost = s.child_costs(p).unwrap();
    assert_eq!(cost["history"].as_array().unwrap().len(), 2);
    assert_eq!(cost["plans"][0]["current"]["disposed_cents"], 0);
    assert_eq!(cost["plans"][0]["current"]["shares"][merged.to_string()], 0);
    assert_eq!(
        s.child_costs(sources[1].0).unwrap()["plans"][0]["current"]["shares"][merged.to_string()],
        2000
    );
}
