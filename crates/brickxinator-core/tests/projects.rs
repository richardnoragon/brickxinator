use brickxinator_core::{ledger::Id, projects::*};
use std::collections::BTreeMap;
fn apply(state: &mut State, action: Action, stock: &BTreeMap<Id, Stock>) {
    *state = state
        .apply(
            &Command {
                id: Id::new(),
                request_key: Id::new().to_string(),
                actor: "test".into(),
                reason: "reviewed test action".into(),
                expected_revision: state.revision,
                action,
            },
            stock,
        )
        .unwrap();
}
fn fixture() -> (State, BTreeMap<Id, Stock>, Id, Id, Id) {
    let mut state = State::default();
    let holding = Id::new();
    let part = Id::new();
    let color = Id::new();
    let version = Id::new();
    let project = Id::new();
    let req = Id::new();
    let stock = BTreeMap::from([(
        holding,
        Stock {
            revision: 1,
            owned: Some(100),
            damaged: Some(2),
            definition_id: Some(part),
            color_id: color,
            copy_id: None,
        },
    )]);
    apply(
        &mut state,
        Action::AddVersion(Version {
            id: version,
            design_id: Id::new(),
            label: "v1".into(),
            author: "Someone else".into(),
            notes: String::new(),
            source: "local file".into(),
            requirements: vec![Requirement {
                id: req,
                definition_id: part,
                color_id: color,
                quantity: 60,
                unit_estimate_cents: Some(3),
                price_snapshot_id: None,
            }],
        }),
        &stock,
    );
    apply(
        &mut state,
        Action::Create(Project {
            id: project,
            name: "Future layout".into(),
            kind: Kind::TrainLayout,
            version_id: version,
            copy_id: None,
            progress: Progress::Planned,
            notes: String::new(),
        }),
        &stock,
    );
    (state, stock, holding, project, req)
}
#[test]
fn verified_reservations_convert_without_changing_owned_stock() {
    let (mut s, stock, h, p, r) = fixture();
    let id = Id::new();
    apply(
        &mut s,
        Action::Claim(Claim {
            id,
            project_id: p,
            requirement_id: r,
            holding_id: h,
            quantity: 60,
            kind: ClaimKind::Reserved,
            verified_revision: None,
        }),
        &stock,
    );
    assert_eq!(s.readiness(p, &stock).unwrap(), Readiness::PotentiallyReady);
    let cmd = Command {
        id: Id::new(),
        request_key: "early-start".into(),
        actor: "test".into(),
        reason: "start".into(),
        expected_revision: s.revision,
        action: Action::Start { project_id: p },
    };
    assert!(s.apply(&cmd, &stock).is_err());
    apply(
        &mut s,
        Action::Verify {
            claim_id: id,
            holding_revision: 1,
        },
        &stock,
    );
    assert_eq!(s.readiness(p, &stock).unwrap(), Readiness::ReadyToBuild);
    apply(&mut s, Action::Start { project_id: p }, &stock);
    assert_eq!(s.claims[&id].kind, ClaimKind::Allocated);
    assert_eq!(s.claimed(h), 60);
    assert_eq!(stock[&h].owned, Some(100));
    apply(
        &mut s,
        Action::Progress {
            project_id: p,
            progress: Progress::Dismantled,
        },
        &stock,
    );
    assert_eq!(s.claimed(h), 60);
}
#[test]
fn claims_block_overuse_and_changed_measurements_invalidate_verification() {
    let (mut s, mut stock, h, p, r) = fixture();
    let id = Id::new();
    apply(
        &mut s,
        Action::Claim(Claim {
            id,
            project_id: p,
            requirement_id: r,
            holding_id: h,
            quantity: 60,
            kind: ClaimKind::Reserved,
            verified_revision: None,
        }),
        &stock,
    );
    apply(
        &mut s,
        Action::Verify {
            claim_id: id,
            holding_revision: 1,
        },
        &stock,
    );
    stock.get_mut(&h).unwrap().revision = 2;
    assert_eq!(s.readiness(p, &stock).unwrap(), Readiness::PotentiallyReady);
    stock.get_mut(&h).unwrap().owned = Some(61);
    assert!(s.validate(&stock).is_err()); // 2 damaged leaves only 59 usable.
    stock.get_mut(&h).unwrap().owned = None;
    assert!(s.validate(&stock).is_err());
}
#[test]
fn versions_are_immutable_and_build_units_do_not_create_verification() {
    let (mut s, stock, _h, p, _r) = fixture();
    let version = s.versions.values().next().unwrap().clone();
    let cmd = Command {
        id: Id::new(),
        request_key: "edit-version".into(),
        actor: "test".into(),
        reason: "edit".into(),
        expected_revision: s.revision,
        action: Action::AddVersion(version),
    };
    assert!(s.apply(&cmd, &stock).is_err());
    apply(
        &mut s,
        Action::Unit(Unit {
            id: Id::new(),
            project_id: p,
            label: "Bag 1".into(),
            kind: UnitKind::Bag,
            progress: UnitProgress::Completed,
        }),
        &stock,
    );
    assert!(s.claims.is_empty());
    assert_eq!(s.coverage(p, &stock).unwrap()[0].potential_gap, 60);
    assert_eq!(s.readiness(p, &stock).unwrap(), Readiness::Planned);
}
#[test]
fn repeated_projects_share_a_version_but_cannot_double_claim() {
    let (mut s, stock, h, p, r) = fixture();
    let mut second = s.projects[&p].clone();
    second.id = Id::new();
    let p2 = second.id;
    apply(&mut s, Action::Create(second), &stock);
    apply(
        &mut s,
        Action::Claim(Claim {
            id: Id::new(),
            project_id: p,
            requirement_id: r,
            holding_id: h,
            quantity: 60,
            kind: ClaimKind::Allocated,
            verified_revision: None,
        }),
        &stock,
    );
    let cmd = Command {
        id: Id::new(),
        request_key: "competing".into(),
        actor: "test".into(),
        reason: "claim".into(),
        expected_revision: s.revision,
        action: Action::Claim(Claim {
            id: Id::new(),
            project_id: p2,
            requirement_id: r,
            holding_id: h,
            quantity: 39,
            kind: ClaimKind::Reserved,
            verified_revision: None,
        }),
    };
    assert!(s.apply(&cmd, &stock).is_err());
    assert_eq!(s.claimed(h), 60);
}

#[test]
fn physical_shortages_are_not_inferred_and_claim_edits_reopen_evidence() {
    let (mut s, stock, h, p, r) = fixture();
    assert_eq!(s.coverage(p, &stock).unwrap()[0].confirmed_missing, 0);
    apply(
        &mut s,
        Action::ConfirmMissing {
            project_id: p,
            requirement_id: r,
            quantity: 7,
        },
        &stock,
    );
    let c = s.coverage(p, &stock).unwrap().remove(0);
    assert_eq!(c.confirmed_missing, 7);
    assert_eq!(c.potential_gap, 53);
    assert_eq!(s.readiness(p, &stock).unwrap(), Readiness::Blocked);
    let all = s.procurement(&stock).unwrap();
    assert_eq!(all[0].confirmed_missing.0, 7);
    apply(
        &mut s,
        Action::Claim(Claim {
            id: Id::new(),
            project_id: p,
            requirement_id: r,
            holding_id: h,
            quantity: 10,
            kind: ClaimKind::Reserved,
            verified_revision: None,
        }),
        &stock,
    );
    assert_eq!(s.coverage(p, &stock).unwrap()[0].confirmed_missing, 0);
}
#[test]
fn selected_price_observations_are_attributed_and_never_reprice_stock() {
    let (mut s, stock, _h, p, r) = fixture();
    let part = s.versions[&s.projects[&p].version_id].requirements[0].definition_id;
    let quote = PriceSnapshot {
        id: Id::new(),
        definition_id: part,
        source: "Manual source A".into(),
        observed_on: "2026-11-22".into(),
        condition: "Used asking price".into(),
        currency: "EUR".into(),
        value_cents: 9,
    };
    apply(&mut s, Action::AddPriceSnapshot(quote.clone()), &stock);
    let mut v = s.versions[&s.projects[&p].version_id].clone();
    v.id = Id::new();
    v.label = "v2 estimate".into();
    v.requirements[0].id = r;
    v.requirements[0].unit_estimate_cents = Some(9);
    v.requirements[0].price_snapshot_id = Some(quote.id);
    apply(&mut s, Action::AddVersion(v.clone()), &stock);
    apply(
        &mut s,
        Action::ChangeVersion {
            project_id: p,
            version_id: v.id,
        },
        &stock,
    );
    assert_eq!(
        s.coverage(p, &stock).unwrap()[0].estimate_cents.unwrap().0,
        540
    );
    assert_eq!(stock.values().next().unwrap().owned, Some(100));
    let mut wrong = v;
    wrong.id = Id::new();
    wrong.requirements[0].unit_estimate_cents = Some(8);
    let cmd = Command {
        id: Id::new(),
        request_key: "bad-estimate".into(),
        actor: "test".into(),
        reason: "test".into(),
        expected_revision: s.revision,
        action: Action::AddVersion(wrong),
    };
    assert!(s.apply(&cmd, &stock).is_err());
}

#[test]
fn partial_claim_transfer_and_release_conserve_physical_stock() {
    let (mut s, stock, h, p, r) = fixture();
    let mut other = s.projects[&p].clone();
    other.id = Id::new();
    let destination = other.id;
    apply(&mut s, Action::Create(other), &stock);
    let original = Id::new();
    let moved = Id::new();
    apply(
        &mut s,
        Action::Claim(Claim {
            id: original,
            project_id: p,
            requirement_id: r,
            holding_id: h,
            quantity: 60,
            kind: ClaimKind::Allocated,
            verified_revision: None,
        }),
        &stock,
    );
    apply(
        &mut s,
        Action::Transfer {
            claim_id: original,
            project_id: destination,
            requirement_id: r,
            quantity: Some(20),
            new_claim_id: Some(moved),
        },
        &stock,
    );
    assert_eq!(s.claims[&original].quantity, 40);
    assert_eq!(s.claims[&moved].quantity, 20);
    assert_eq!(s.claimed(h), 60);
    apply(
        &mut s,
        Action::Release {
            claim_id: moved,
            quantity: Some(5),
        },
        &stock,
    );
    assert_eq!(s.claimed(h), 55);
    assert_eq!(stock[&h].owned, Some(100));
}
