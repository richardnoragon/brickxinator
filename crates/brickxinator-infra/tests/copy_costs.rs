use brickxinator_core::{
    child_costs::Command,
    ledger::{Body, Id, Operation},
    physical::*,
    projects,
    purchases::PurchaseCommand,
    verification,
};
use brickxinator_infra::{exchange, sqlite::Store};
fn fixture() -> (tempfile::TempDir, Store, Id, Id, Id, Id, Id, Id) {
    let root = tempfile::tempdir().unwrap();
    let mut s = Store::create(root.path().join("collection.sqlite")).unwrap();
    let p = Id::new();
    let line = Id::new();
    let body = serde_json::json!({"Create":{"purchase_id":p,"draft":{"shipping_cents":0,"fees_cents":0,"notes":"","lines":[{"id":line,"kind":"Set","description":"Physical set","copy_count":1,"pieces":"Unknown","weight_mg":"Unknown","paid_total_cents":10000,"assembly":"Loose","volume_ml":1000}]}}});
    let pc = |body| {
        serde_json::from_value::<PurchaseCommand>(serde_json::json!({"id":Id::new(),"request_key":Id::new().to_string(),"actor":"test","reason":"actual acquisition","body":body})).unwrap()
    };
    s.execute_purchase(pc(body)).unwrap();
    let receipt = s
        .execute_purchase(pc(
            serde_json::json!({"Receive":{"purchase_id":p,"expected_revision":1}}),
        ))
        .unwrap();
    let copy = *receipt.receipt.unwrap().set_copies.keys().next().unwrap();
    let manufacturer = s
        .settings()
        .unwrap()
        .values()
        .find(|r| {
            matches!(
                r.kind,
                brickxinator_core::settings::Kind::Vocabulary {
                    vocabulary: brickxinator_core::settings::Vocabulary::Manufacturer,
                    system_unknown: true
                }
            )
        })
        .unwrap()
        .id;
    let part = Id::new();
    s.execute_catalog(serde_json::from_value(serde_json::json!({"id":Id::new(),"request_key":Id::new().to_string(),"actor":"test","reason":"part identity","action":{"Create":{"id":part,"kind":"Part","details":{"manufacturer_id":manufacturer,"number":"3001","name":"Brick"}}}})).unwrap()).unwrap();
    let color = s.default_inventory_links().unwrap().color_id;
    let holding = Id::new();
    let vline = Id::new();
    let v = |rev, action| verification::Command {
        id: Id::new(),
        request_key: Id::new().to_string(),
        actor: "test".into(),
        reason: "physical count".into(),
        copy_id: copy,
        expected_revision: rev,
        action,
    };
    s.execute_verification(v(
        0,
        verification::Action::AddLine {
            line_id: vline,
            holding_id: holding,
            definition_id: Some(part),
            color_id: color,
            expected: Some(100),
        },
    ))
    .unwrap();
    s.execute_verification(v(
        1,
        verification::Action::Count {
            line_id: vline,
            total: 100,
        },
    ))
    .unwrap();
    (root, s, p, line, copy, holding, part, color)
}
fn allocation(s: &Store, p: Id, line: Id, copy: Id, holding: Id) -> Command {
    let v = s.child_costs(p).unwrap();
    serde_json::from_value(serde_json::json!({"id":Id::new(),"request_key":Id::new().to_string(),"actor":"test","reason":"shared source budget","effective_on":"2026-11-23","purchase_id":p,"invoice_line_id":line,"component":"Item","expected_revision":0,"expected_inventory_sequence":v["inventory_sequence"],"expected_purchase_revision":2,"expected_fulfilment_revision":0,"expected_shipping_revision":0,"method":"Manual","allocated_cents":10000,"targets":{holding.to_string():3000},"copy_targets":{copy.to_string():7000},"copy_piece_bases":{}})).unwrap()
}
fn physical(s: &mut Store, body: Body) -> Id {
    let o = Operation {
        id: Id::new(),
        request_key: Id::new().to_string(),
        actor: "test".into(),
        reason: "reviewed physical event".into(),
        body,
    };
    let id = o.id;
    let p = s.preview_physical(o.clone()).unwrap();
    s.execute_physical(o, p["expected_profile_sequence"].as_i64().unwrap())
        .unwrap();
    id
}
#[test]
fn copy_and_parts_share_one_budget_and_safe_disposal_correction_restores_both() {
    let (root, mut s, p, line, copy, h, _, _) = fixture();
    let before = s.rebuild().unwrap().operations().len();
    let c = allocation(&s, p, line, copy, h);
    let plan = s.execute_child_cost(c.clone()).unwrap();
    assert_eq!(plan.copy_shares[&copy], 7000);
    assert_eq!(s.execute_child_cost(c).unwrap(), plan);
    assert_eq!(s.rebuild().unwrap().operations().len(), before);
    let view = s.cost_rollups().unwrap();
    assert_eq!(view["reviewed_owned"]["known_cents"], "10000");
    assert_eq!(
        view["copies"][copy.to_string()]["direct"]["known_cents"],
        "7000"
    );
    assert_eq!(
        view["copies"][copy.to_string()]["allocated_parts"]["known_cents"],
        "3000"
    );
    let rev = s.rebuild().unwrap().holdings()[&h].revision;
    let sold = physical(
        &mut s,
        Body::Lifecycle(Lifecycle {
            copy_id: copy,
            expected_revision: 0,
            status: Status::Sold,
            effective_on: "2026-11-23".into(),
            proceeds_cents: Some(12000),
            recipient: None,
            notes: "complete sale".into(),
            parts: vec![Disposition {
                holding_id: h,
                expected_revision: rev,
                handling: Handling::TransferAll,
            }],
            corrects: None,
        }),
    );
    assert_eq!(
        s.cost_rollups().unwrap()["reviewed_owned"]["known_cents"],
        "0"
    );
    assert_eq!(
        s.cost_rollups().unwrap()["disposed"]["known_cents"],
        "10000"
    );
    physical(
        &mut s,
        Body::Lifecycle(Lifecycle {
            copy_id: copy,
            expected_revision: 1,
            status: Status::Active,
            effective_on: "2026-11-23".into(),
            proceeds_cents: None,
            recipient: None,
            notes: "sale entered in error".into(),
            parts: vec![],
            corrects: Some(sold),
        }),
    );
    assert_eq!(
        s.cost_rollups().unwrap()["reviewed_owned"]["known_cents"],
        "10000"
    );
    let snapshot = s.cost_rollups().unwrap();
    drop(s);
    let native = root.path().join("native");
    exchange::export(root.path().join("collection.sqlite"), &native).unwrap();
    let target = root.path().join("restored.sqlite");
    drop(Store::create(&target).unwrap());
    exchange::import(&target, &native, root.path().join("safe.zip")).unwrap();
    assert_eq!(
        Store::open(target).unwrap().cost_rollups().unwrap(),
        snapshot
    );
}
#[test]
fn copy_piece_count_basis_is_explicit_and_overspending_cannot_mutate_stock() {
    let (_root, mut s, p, line, copy, h, _, _) = fixture();
    let mut c = allocation(&s, p, line, copy, h);
    c.allocated_cents = 10001;
    c.copy_targets.insert(copy, Some(7001));
    assert!(s.execute_child_cost(c).is_err());
    let mut c = allocation(&s, p, line, copy, h);
    c.method = brickxinator_core::child_costs::AllocationMethod::PieceCount;
    c.targets.clear();
    c.copy_targets.insert(copy, None);
    assert!(s.preview_child_cost(c.clone()).is_err());
    c.copy_piece_bases.insert(copy, 100);
    let plan = s.execute_child_cost(c).unwrap();
    assert_eq!(plan.copy_shares[&copy], 10000);
    assert_eq!(
        s.rebuild().unwrap().holdings()[&h].pieces.owned(),
        brickxinator_core::inventory::Measure::Known(100)
    );
}
#[test]
fn project_rollups_allocate_cost_by_actual_claims_without_recounting_copy_cost() {
    let (_root, mut s, p, line, copy, h, part, color) = fixture();
    let c = allocation(&s, p, line, copy, h);
    s.execute_child_cost(c).unwrap();
    let rev = s.rebuild().unwrap().holdings()[&h].revision;
    physical(
        &mut s,
        Body::Detach {
            holding_id: h,
            expected_revision: rev,
        },
    );
    let version = Id::new();
    let req = Id::new();
    let cmd = |rev, action| projects::Command {
        id: Id::new(),
        request_key: Id::new().to_string(),
        actor: "test".into(),
        reason: "real allocation".into(),
        expected_revision: rev,
        action,
    };
    s.execute_project(cmd(
        0,
        projects::Action::AddVersion(projects::Version {
            id: version,
            design_id: Id::new(),
            label: "BOM".into(),
            author: "".into(),
            notes: "".into(),
            source: "manual".into(),
            requirements: vec![projects::Requirement {
                id: req,
                definition_id: part,
                color_id: color,
                quantity: 40,
                unit_estimate_cents: None,
                price_snapshot_id: None,
            }],
        }),
    ))
    .unwrap();
    let a = Id::new();
    let b = Id::new();
    let mut revision = 1;
    for (id, qty, linked) in [(a, 40, Some(copy)), (b, 20, None)] {
        s.execute_project(cmd(
            revision,
            projects::Action::Create(projects::Project {
                id,
                name: format!("Build {qty}"),
                kind: if linked.is_some() {
                    projects::Kind::OfficialSetBuild
                } else {
                    projects::Kind::Moc
                },
                version_id: version,
                copy_id: linked,
                progress: projects::Progress::Planned,
                notes: "".into(),
            }),
        ))
        .unwrap();
        revision += 1;
        s.execute_project(cmd(
            revision,
            projects::Action::Claim(projects::Claim {
                id: Id::new(),
                project_id: id,
                requirement_id: req,
                holding_id: h,
                quantity: qty,
                kind: projects::ClaimKind::Allocated,
                verified_revision: None,
            }),
        ))
        .unwrap();
        revision += 1;
    }
    let rollup = s.cost_rollups().unwrap();
    assert_eq!(
        rollup["projects"][a.to_string()]["allocated_parts"]["known_cents"],
        "1200"
    );
    assert_eq!(
        rollup["projects"][b.to_string()]["allocated_parts"]["known_cents"],
        "600"
    );
    assert_eq!(
        rollup["copies"][copy.to_string()]["allocated_parts"]["known_cents"],
        "1200"
    );
    assert_eq!(rollup["reviewed_owned"]["known_cents"], "10000");
}
#[test]
fn refunded_disposed_copy_can_receive_a_reviewed_cost_only_correction_without_fake_targets() {
    use brickxinator_core::{adjustments, finance, fulfilment};
    let (_root, mut s, p, line, copy, h, _, _) = fixture();
    let c = allocation(&s, p, line, copy, h);
    s.execute_child_cost(c).unwrap();
    let rev = s.rebuild().unwrap().holdings()[&h].revision;
    physical(
        &mut s,
        Body::Lifecycle(Lifecycle {
            copy_id: copy,
            expected_revision: 0,
            status: Status::Returned,
            effective_on: "2026-11-23".into(),
            proceeds_cents: None,
            recipient: None,
            notes: "physical handover".into(),
            parts: vec![Disposition {
                holding_id: h,
                expected_revision: rev,
                handling: Handling::TransferAll,
            }],
            corrects: None,
        }),
    );
    let refund = fulfilment::Command {
        id: Id::new(),
        request_key: Id::new().to_string(),
        actor: "test".into(),
        reason: "full supplier refund".into(),
        purchase_id: p,
        expected_revision: 0,
        expected_purchase_revision: 2,
        action: fulfilment::Action::Financial(adjustments::Action::Record(adjustments::Entry {
            id: Id::new(),
            kind: adjustments::Kind::Refund,
            label: "Full refund".into(),
            effective_on: "2026-11-23".into(),
            amount_cents: 10000,
            method: finance::Method::PriceProportional,
            payment_method: None,
            warranty_id: None,
            return_id: None,
        })),
    };
    let preview = s.preview_fulfilment(refund.clone()).unwrap();
    s.execute_fulfilment(
        refund,
        preview["expected_inventory_sequence"].as_u64().unwrap() as usize,
        preview["expected_profile_sequence"].as_i64().unwrap(),
    )
    .unwrap();
    let mut c = allocation(&s, p, line, copy, h);
    c.expected_revision = 1;
    c.expected_fulfilment_revision = 1;
    c.allocated_cents = 0;
    c.targets.clear();
    c.copy_targets.clear();
    c.reviewed_disposed_cents = Some(0);
    s.execute_child_cost(c).unwrap();
    assert_eq!(s.cost_rollups().unwrap()["disposed"]["known_cents"], "0");
    assert_eq!(
        s.rebuild().unwrap().lifecycle(copy).status,
        Status::Returned
    );
}
