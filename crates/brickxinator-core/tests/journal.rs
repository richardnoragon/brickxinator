use brickxinator_core::{
    inventory::{AssemblyState, Measure},
    journal::{Action, Command, Journal, JournalError, MeasureChanges},
};

fn command(key: &str, revision: u64, owned: Action, damaged: Action) -> Command {
    Command {
        request_key: key.into(),
        expected_revision: revision,
        actor: "local-user".into(),
        reason: "Synthetic stock observation".into(),
        pieces: MeasureChanges { owned, damaged },
        weight_mg: MeasureChanges::default(),
        assembly: None,
    }
}

#[test]
fn retry_returns_original_result_without_reapplying_stock() {
    let mut journal = Journal::default();
    let receive = command("first", 0, Action::SetMeasured(100), Action::SetMeasured(0));
    let original = journal.append(receive.clone()).unwrap();
    journal
        .append(command("lost", 1, Action::Delta(-5), Action::Unchanged))
        .unwrap();
    assert_eq!(journal.append(receive.clone()).unwrap(), original);
    assert_eq!(journal.current().pieces.owned(), Measure::Known(95));
    assert_eq!(journal.entries().len(), 2);
    let mut conflicting = receive;
    conflicting.pieces.owned = Action::SetMeasured(200);
    assert_eq!(
        journal.append(conflicting),
        Err(JournalError::RequestKeyConflict)
    );
    assert_eq!(
        Journal::replay(journal.entries()).unwrap(),
        journal.current()
    );
}

#[test]
fn failed_commands_leave_both_measures_and_history_unchanged() {
    let mut journal = Journal::default();
    journal
        .append(command(
            "observe",
            0,
            Action::SetMeasured(50),
            Action::SetMeasured(0),
        ))
        .unwrap();
    let before = journal.current();
    let mut invalid = command("invalid", 1, Action::Delta(-51), Action::Unchanged);
    invalid.weight_mg.owned = Action::SetMeasured(1000);
    assert_eq!(journal.append(invalid), Err(JournalError::OutOfRange));
    assert_eq!(journal.current(), before);
    assert_eq!(journal.entries().len(), 1);
    assert_eq!(
        journal.append(command("stale", 0, Action::Delta(1), Action::Unchanged)),
        Err(JournalError::StaleRevision)
    );
    assert_eq!(journal.current(), before);
}

#[test]
fn unknown_is_never_an_implicit_zero_and_absolute_observations_are_not_summed() {
    let mut journal = Journal::default();
    assert_eq!(
        journal.append(command("delta", 0, Action::Delta(7), Action::Unchanged)),
        Err(JournalError::UnknownDelta)
    );
    for (revision, key, count) in [(0, "count-7", 7), (1, "repeat-7", 7), (2, "count-9", 9)] {
        journal
            .append(command(
                key,
                revision,
                Action::SetMeasured(count),
                Action::SetMeasured(0),
            ))
            .unwrap();
        assert_eq!(journal.current().pieces.owned(), Measure::Known(count));
    }
    assert_eq!(
        Journal::replay(journal.entries()).unwrap(),
        journal.current()
    );
}

#[test]
fn damage_disposal_and_assembly_replay_without_duplicate_ownership() {
    let mut journal = Journal::default();
    journal
        .append(command(
            "receipt",
            0,
            Action::SetMeasured(100),
            Action::SetMeasured(0),
        ))
        .unwrap();
    journal
        .append(command("broken", 1, Action::Unchanged, Action::Delta(2)))
        .unwrap();
    journal
        .append(command("donated", 2, Action::Delta(-5), Action::Unchanged))
        .unwrap();
    assert_eq!(journal.current().pieces.owned(), Measure::Known(95));
    assert_eq!(
        journal.current().pieces.available(AssemblyState::Loose),
        Measure::Known(93)
    );
    for (revision, state) in [
        (3, AssemblyState::Assembled),
        (4, AssemblyState::PartiallyAssembled),
        (5, AssemblyState::Loose),
    ] {
        let mut change = command(
            &format!("assembly-{revision}"),
            revision,
            Action::Unchanged,
            Action::Unchanged,
        );
        change.assembly = Some(state);
        let result = journal.append(change).unwrap();
        assert_eq!(result.pieces.owned(), Measure::Known(95));
        assert_eq!(
            result.pieces.available(state),
            Measure::Known(if state == AssemblyState::Loose { 93 } else { 0 })
        );
    }
    assert_eq!(
        Journal::replay(journal.entries()).unwrap(),
        journal.current()
    );
}

#[test]
fn invalidating_residual_weight_preserves_original_observation() {
    let mut journal = Journal::default();
    let mut receipt = command("weight", 0, Action::Unchanged, Action::Unchanged);
    receipt.weight_mg = MeasureChanges {
        owned: Action::SetMeasured(5_000_000),
        damaged: Action::SetMeasured(0),
    };
    journal.append(receipt).unwrap();
    let mut extraction = command("unweighed", 1, Action::Unchanged, Action::Unchanged);
    extraction.weight_mg.owned = Action::Invalidate;
    journal.append(extraction).unwrap();
    assert_eq!(journal.current().weight_mg.owned(), Measure::Unknown);
    assert_eq!(journal.current().pieces.owned(), Measure::Unknown);
    assert_eq!(
        journal.entries()[0].command().weight_mg.owned,
        Action::SetMeasured(5_000_000)
    );
    assert_eq!(
        Journal::replay(journal.entries()).unwrap(),
        journal.current()
    );
    assert!(Journal::replay(&journal.entries()[1..]).is_err());
}

#[test]
fn missing_audit_overflow_and_impossible_damage_are_rejected() {
    let mut journal = Journal::default();
    let mut unaudited = command(
        "no-reason",
        0,
        Action::SetMeasured(1),
        Action::SetMeasured(0),
    );
    unaudited.reason = " ".into();
    assert_eq!(
        journal.append(unaudited),
        Err(JournalError::MissingAuditContext)
    );
    assert_eq!(
        journal.append(command(
            "damage",
            0,
            Action::SetMeasured(1),
            Action::SetMeasured(2)
        )),
        Err(JournalError::DamageExceedsOwnership)
    );
    journal
        .append(command(
            "maximum",
            0,
            Action::SetMeasured(u64::MAX),
            Action::SetMeasured(0),
        ))
        .unwrap();
    assert_eq!(
        journal.append(command("overflow", 1, Action::Delta(1), Action::Unchanged)),
        Err(JournalError::OutOfRange)
    );
    assert_eq!(journal.entries().len(), 1);
}
