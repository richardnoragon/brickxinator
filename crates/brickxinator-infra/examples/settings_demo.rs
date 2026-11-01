//! Synthetic managed vocabulary and nested location demonstration.
use brickxinator_core::{
    ledger::Id,
    settings::{Action, Command, Vocabulary},
};
use brickxinator_infra::sqlite::{Result, Store};

fn command(key: &str, action: Action) -> Command {
    Command {
        id: Id::new(),
        request_key: key.into(),
        actor: "demo".into(),
        reason: String::new(),
        accept_name_warnings: false,
        action,
    }
}
fn main() -> Result<()> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: settings_demo <NEW-database-path>")?;
    let mut store = Store::create(&path)?;
    let brand = Id::new();
    let room = Id::new();
    let cabinet = Id::new();
    let drawer = Id::new();
    store.execute_settings(command(
        "brand",
        Action::CreateVocabulary {
            id: brand,
            vocabulary: Vocabulary::Manufacturer,
            name: "LEGO".into(),
        },
    ))?;
    for (key, id, name, parent_id) in [
        ("room", room, "Workshop", None),
        ("cabinet", cabinet, "Cabinet 4", Some(room)),
        ("drawer", drawer, "Drawer 14", Some(cabinet)),
    ] {
        store.execute_settings(command(
            key,
            Action::CreateLocation {
                id,
                parent_id,
                name: name.into(),
            },
        ))?;
    }
    store.execute_settings(command(
        "rename",
        Action::Rename {
            id: drawer,
            expected_revision: 1,
            name: "Drawer 15".into(),
        },
    ))?;
    store.execute_settings(command(
        "archive",
        Action::Archive {
            id: cabinet,
            expected_revision: 1,
            archived: true,
        },
    ))?;
    drop(store);
    let store = Store::open(path)?;
    println!(
        "Reopened {} settings records; {} currently selectable",
        store.settings()?.len(),
        store.selectable_settings()?.len()
    );
    println!(
        "Drawer ID {drawer} retains name {:?}",
        store.settings()?[&drawer].name
    );
    println!(
        "Drawer history: {} revisions",
        store.settings_history(drawer)?.len()
    );
    println!(
        "Inventory operations after settings edits: {}",
        store.rebuild()?.operations().len()
    );
    Ok(())
}
