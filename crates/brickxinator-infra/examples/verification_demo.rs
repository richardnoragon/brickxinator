//! Synthetic explicit verification, shortage, correction and assembly workflow.
use brickxinator_core::{
    inventory::{AssemblyState, Measure},
    ledger::Id,
    purchases::{Draft, InvoiceLine, LineKind, PurchaseAction, PurchaseCommand},
    settings::{Kind, Vocabulary},
    verification::{Action, Command},
};
use brickxinator_infra::sqlite::{Result, Store};
fn command(key: &str, copy_id: Id, expected_revision: u64, action: Action) -> Command {
    Command {
        id: Id::new(),
        request_key: key.into(),
        actor: "demo".into(),
        reason: "Synthetic verification workflow".into(),
        copy_id,
        expected_revision,
        action,
    }
}
fn main() -> Result<()> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: verification_demo <NEW-database-path>")?;
    let mut store = Store::create(&path)?;
    let purchase = Id::new();
    store.execute_purchase(PurchaseCommand {
        id: Id::new(),
        request_key: "create".into(),
        actor: "demo".into(),
        reason: String::new(),
        body: PurchaseAction::Create {
            purchase_id: purchase,
            draft: Draft {
                lines: vec![InvoiceLine {
                    source_url: None,
                    notes: String::new(),
                    id: Id::new(),
                    kind: LineKind::Set,
                    description: Some("Synthetic assembled set".into()),
                    set_number: Some("7838".into()),
                    copy_count: Some(1),
                    pieces: Measure::Known(451),
                    weight_mg: Measure::Unknown,
                    paid_total_cents: None,
                    assembly: AssemblyState::Assembled,
                }],
                ..Draft::default()
            },
        },
    })?;
    let receipt = store.execute_purchase(PurchaseCommand {
        id: Id::new(),
        request_key: "receive".into(),
        actor: "demo".into(),
        reason: String::new(),
        body: PurchaseAction::Receive {
            purchase_id: purchase,
            expected_revision: 1,
        },
    })?;
    let copy = *receipt
        .receipt
        .ok_or("missing receipt")?
        .set_copies
        .keys()
        .next()
        .ok_or("missing copy")?;
    let color = store
        .settings()?
        .values()
        .find(|r| {
            matches!(
                r.kind,
                Kind::Vocabulary {
                    vocabulary: Vocabulary::Color,
                    system_unknown: true
                }
            )
        })
        .ok_or("missing Unknown color")?
        .id;
    let line = Id::new();
    store.execute_verification(command(
        "expected",
        copy,
        0,
        Action::AddLine {
            line_id: line,
            holding_id: Id::new(),
            definition_id: None,
            color_id: color,
            expected: Some(10),
        },
    ))?;
    println!(
        "Expected checklist creates {} holdings",
        store.rebuild()?.holdings().len()
    );
    let count = command(
        "count",
        copy,
        1,
        Action::Count {
            line_id: line,
            total: 7,
        },
    );
    store.execute_verification(count.clone())?;
    println!(
        "Before finishing: {:?}",
        store.set_verification(copy)?.lines[&line].difference()
    );
    store.execute_verification(command("finish", copy, 2, Action::Finish { line_id: line }))?;
    store.execute_verification(command("complete", copy, 3, Action::Complete))?;
    println!(
        "Finished: {:?}; status {:?}",
        store.set_verification(copy)?.lines[&line].difference(),
        store.set_verification(copy)?.status()
    );
    drop(store);
    let mut store = Store::open(path)?;
    store.execute_verification(count)?;
    store.execute_verification(command(
        "correct-count",
        copy,
        4,
        Action::Count {
            line_id: line,
            total: 9,
        },
    ))?;
    store.execute_verification(command(
        "disassemble",
        copy,
        5,
        Action::Assembly {
            state: AssemblyState::Loose,
        },
    ))?;
    let content = store.set_contents(copy)?;
    let holding = content[0].holding;
    println!(
        "After reopen/retry/correction: owned {:?}, available {:?}, verification {:?}",
        holding.pieces.owned(),
        holding.pieces.available(holding.assembly),
        store.set_verification(copy)?.status()
    );
    println!(
        "Audited verification revisions: {}",
        store.verification_history(copy)?.len()
    );
    Ok(())
}
