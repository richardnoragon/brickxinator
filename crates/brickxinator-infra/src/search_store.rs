//! Rebuildable SQLite FTS/read cache. Current journal and audited metadata remain
//! authoritative; clean-cache pagination does not replay the inventory journal.
use crate::{
    catalog_store::{default_links, load_catalog, load_profiles},
    purchase_store::read_purchase,
    settings_store::load_settings,
    sqlite::{Result, Store, has_search, load},
};
use brickxinator_core::{
    catalog::{Definition, normalized_number},
    inventory::Measure,
    ledger::{Body, Id},
    profiles::{Links, Target as ProfileTarget},
    purchases::{InvoiceLine, LineKind, Purchase, Status},
    settings::{Kind as SettingKind, Record, Vocabulary},
    verification,
    views::{
        Availability, Dashboard, InventoryRow, InventoryTotals, Kind, Named, Page, PurchaseCard,
        Search, Target,
    },
};
use rusqlite::{Connection, TransactionBehavior, params, params_from_iter, types::Value};
use std::collections::BTreeMap;

const CACHE_SCHEMA: &str = include_str!("../../../migrations/migration_007.sql");

impl Store {
    pub fn search_inventory(&mut self, query: &Search) -> Result<Page> {
        if query.limit == 0 || query.limit > 200 || query.text.chars().count() > 512 {
            return Err("search limit must be 1..200 and text at most 512 characters".into());
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure(&tx, false)?;
        let (sql, values) = selection(query)?;
        let mut totals = InventoryTotals::default();
        let mut total_matches = 0;
        {
            let mut statement = tx.prepare(&sql)?;
            let mut rows = statement.query(params_from_iter(values.iter()))?;
            while let Some(row) = rows.next()? {
                let record: InventoryRow = serde_json::from_str(&row.get::<_, String>(0)?)?;
                totals.add(&record);
                total_matches += 1;
            }
        }
        let mut page_values = values;
        page_values.push(Value::Text(query.text.trim().to_lowercase()));
        page_values.push(Value::Integer(i64::from(query.limit)));
        page_values.push(Value::Integer(i64::from(query.offset)));
        let page_sql = format!(
            "{sql} ORDER BY CASE WHEN d.normalized_number=? THEN 0 ELSE 1 END,d.target_kind,d.target_id LIMIT ? OFFSET ?"
        );
        let records = {
            let mut statement = tx.prepare(&page_sql)?;
            let rows = statement.query_map(params_from_iter(page_values.iter()), |row| {
                row.get::<_, String>(0)
            })?;
            rows.map(|json| Ok(serde_json::from_str(&json?)?))
                .collect::<Result<Vec<InventoryRow>>>()?
        };
        tx.commit()?;
        Ok(Page {
            rows: records,
            total_matches,
            totals,
            offset: query.offset,
            limit: query.limit,
        })
    }

    pub fn dashboard(&mut self) -> Result<Dashboard> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure(&tx, false)?;
        let json: String = tx.query_row(
            "SELECT dashboard_json FROM inventory_search_state WHERE id=1",
            [],
            |r| r.get(0),
        )?;
        let dashboard = serde_json::from_str(&json)?;
        tx.commit()?;
        Ok(dashboard)
    }

    pub fn rebuild_inventory_search(&mut self) -> Result<u64> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure(&tx, true)?;
        let count = u64::try_from(tx.query_row(
            "SELECT count(*) FROM inventory_search_document",
            [],
            |r| r.get::<_, i64>(0),
        )?)?;
        tx.commit()?;
        Ok(count)
    }
}

fn ensure(connection: &Connection, force: bool) -> Result<()> {
    if !has_search(connection)? {
        return Err(
            "search/dashboard need schema 7; automatic upgrades are not implemented".into(),
        );
    }
    let missing:i64=connection.query_row("SELECT 3-count(*) FROM sqlite_schema WHERE name IN ('inventory_search_document','inventory_search_fts','inventory_search_state')",[],|r|r.get(0))?;
    if force {
        connection.execute("DROP TABLE IF EXISTS inventory_search_fts", [])?;
    }
    if force || missing != 0 {
        connection.execute_batch(CACHE_SCHEMA)?;
    }
    connection.execute(
        "INSERT OR IGNORE INTO inventory_search_state(id,dirty,format_version) VALUES(1,1,1)",
        [],
    )?;
    let (dirty, version, empty): (bool, i64, bool) = connection.query_row(
        "SELECT dirty,format_version,dashboard_json IS NULL FROM inventory_search_state WHERE id=1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    if force || missing != 0 || dirty || version != 2 || empty {
        rebuild(connection)?;
    }
    Ok(())
}

fn selection(query: &Search) -> Result<(String, Vec<Value>)> {
    let mut clauses = vec!["1=1".to_string()];
    let mut values = vec![];
    if let Some(assembly) = query.assembly {
        clauses.push("json_extract(d.snapshot_json,'$.assembly')=?".into());
        values.push(Value::Text(
            match assembly {
                brickxinator_core::inventory::AssemblyState::Loose => "Loose",
                brickxinator_core::inventory::AssemblyState::Assembled => "Assembled",
                brickxinator_core::inventory::AssemblyState::PartiallyAssembled => {
                    "PartiallyAssembled"
                }
            }
            .into(),
        ));
    }
    if let Some(archived) = query.archived {
        clauses.push("json_extract(d.snapshot_json,'$.archived')=?".into());
        values.push(Value::Integer(i64::from(archived)));
    }
    if let Some(seller) = &query.seller {
        if seller.len() > 512 {
            return Err("seller filter exceeds 512 bytes".into());
        }
        clauses.push(
            "instr(lower(coalesce(json_extract(d.snapshot_json,'$.seller'),'')),lower(?))>0".into(),
        );
        values.push(Value::Text(seller.clone()));
    }
    for (date, comparison) in [
        (&query.purchased_from, ">="),
        (&query.purchased_through, "<="),
    ] {
        if let Some(date) = date {
            if !brickxinator_core::purchases::valid_date(date) {
                return Err("purchase date filter must be a valid YYYY-MM-DD date".into());
            }
            clauses.push(format!("EXISTS(SELECT 1 FROM purchase p WHERE p.id=json_extract(d.snapshot_json,'$.purchase_id') AND json_extract(p.snapshot_json,'$.draft.purchased_on'){comparison}?)"));
            values.push(Value::Text(date.clone()));
        }
    }
    if let (Some(from), Some(through)) = (&query.purchased_from, &query.purchased_through)
        && from > through
    {
        return Err("purchase date range is reversed".into());
    }
    if !query.text.trim().is_empty() {
        let exact = query.text.trim().to_lowercase();
        let words: Vec<_> = query
            .text
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .collect();
        if words.len() > 32 {
            return Err("search supports at most 32 literal terms".into());
        }
        if words.is_empty() {
            clauses.push("(d.normalized_number=? OR d.target_id=?)".into());
            values.extend([Value::Text(exact.clone()), Value::Text(exact)]);
        } else {
            let expression = words
                .iter()
                .map(|word| format!("\"{}\"", word.replace('"', "\"\"")))
                .collect::<Vec<_>>()
                .join(" AND ");
            clauses.push("(d.normalized_number=? OR d.target_id=? OR d.rowid IN (SELECT rowid FROM inventory_search_fts WHERE inventory_search_fts MATCH ?))".into());
            values.extend([
                Value::Text(exact.clone()),
                Value::Text(exact),
                Value::Text(expression),
            ]);
        }
    }
    if let Some(number) = &query.number {
        let number =
            normalized_number(&Some(number.clone())).ok_or("number filter cannot be blank")?;
        clauses.push("d.normalized_number=?".into());
        values.push(Value::Text(number));
    }
    for (column, id) in [
        ("manufacturer_id", query.manufacturer_id),
        ("color_id", query.color_id),
        ("condition_id", query.condition_id),
    ] {
        if let Some(id) = id {
            clauses.push(format!("d.{column}=?"));
            values.push(Value::Text(id.to_string()));
        }
    }
    if let Some(id) = query.location_id {
        clauses.push(
            if query.include_descendants {
                "EXISTS(SELECT 1 FROM json_each(d.location_ids_json) WHERE value=?)"
            } else {
                "d.location_id=?"
            }
            .into(),
        );
        values.push(Value::Text(id.to_string()));
    }
    if let Some(availability) = query.availability {
        clauses.push(format!(
            "d.{}=1",
            match availability {
                Availability::Available => "available",
                Availability::InAssembledSet => "assembled",
                Availability::BrokenDamaged => "damaged",
                Availability::Unknown => "unknown_measure",
            }
        ));
    }
    if let Some(kind) = query.kind {
        clauses.push("d.row_kind=?".into());
        values.push(Value::Text(kind_name(kind).into()));
    }
    for (column, value) in [
        ("incomplete", query.incomplete),
        ("archived_reference", query.archived_references),
    ] {
        if let Some(value) = value {
            clauses.push(format!("d.{column}=?"));
            values.push(Value::Integer(i64::from(value)));
        }
    }
    Ok((
        format!(
            "SELECT d.snapshot_json FROM inventory_search_document d WHERE {}",
            clauses.join(" AND ")
        ),
        values,
    ))
}

fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::LooseBatch => "LooseBatch",
        Kind::MixedLot => "MixedLot",
        Kind::IdentifiedBatch => "IdentifiedBatch",
        Kind::SetContent => "SetContent",
        Kind::SetCopy => "SetCopy",
        Kind::UnquantifiedSetLine => "UnquantifiedSetLine",
        Kind::OpeningHolding => "OpeningHolding",
    }
}
fn target_name(target: Target) -> &'static str {
    match target {
        Target::Holding(_) => "Holding",
        Target::SetCopy(_) => "SetCopy",
        Target::UnquantifiedSetLine(_) => "UnquantifiedSetLine",
    }
}
fn parse_id(value: String) -> Result<Id> {
    Ok(serde_json::from_value(serde_json::Value::String(value))?)
}
fn named(settings: &BTreeMap<Id, Record>, id: Id) -> Result<Named> {
    let r = settings.get(&id).ok_or("missing managed reference")?;
    Ok(Named {
        id,
        name: r.name.clone(),
        archived: r.archived,
    })
}
fn path(settings: &BTreeMap<Id, Record>, location: Option<Id>) -> Result<Vec<Named>> {
    let mut result = vec![];
    let mut next = location;
    while let Some(id) = next {
        let record = settings.get(&id).ok_or("missing location")?;
        result.push(named(settings, id)?);
        let SettingKind::Location { parent_id } = record.kind else {
            return Err("invalid location reference".into());
        };
        next = parent_id;
    }
    result.reverse();
    Ok(result)
}
fn unknown(settings: &BTreeMap<Id, Record>, vocabulary: Vocabulary) -> Result<Id> {
    settings.values().find(|r|matches!(r.kind,SettingKind::Vocabulary {vocabulary:v,system_unknown:true} if v==vocabulary)).map(|r|r.id).ok_or_else(||"missing protected Unknown".into())
}

struct Source {
    purchase: Id,
    line: InvoiceLine,
}
struct Context<'a> {
    settings: &'a BTreeMap<Id, Record>,
    definitions: &'a BTreeMap<Id, Definition>,
    purchases: &'a BTreeMap<Id, Purchase>,
    lines: &'a BTreeMap<Id, Source>,
    unknown_manufacturer: Id,
    unknown_color: Id,
    unknown_condition: Id,
}
impl Context<'_> {
    fn row(
        &self,
        target: Target,
        kind: Kind,
        links: &Links,
        source: Option<Id>,
    ) -> Result<(InventoryRow, String)> {
        let source = source.and_then(|id| self.lines.get(&id));
        let purchase = source.and_then(|s| self.purchases.get(&s.purchase));
        let definition = links.definition_id.and_then(|id| self.definitions.get(&id));
        let manufacturer = named(
            self.settings,
            definition.map_or(self.unknown_manufacturer, |d| d.details.manufacturer_id),
        )?;
        let color = named(self.settings, links.color_id)?;
        let condition = named(self.settings, links.condition_id)?;
        let location = path(self.settings, links.location_id)?;
        let number = definition
            .and_then(|d| d.details.number.clone())
            .or_else(|| {
                if matches!(target, Target::SetCopy(_) | Target::UnquantifiedSetLine(_)) {
                    source.and_then(|s| s.line.set_number.clone())
                } else {
                    None
                }
            });
        let label = definition
            .and_then(|d| d.details.name.clone())
            .or_else(|| source.and_then(|s| s.line.description.clone()))
            .unwrap_or_else(|| kind_name(kind).into());
        let text = [
            definition
                .and_then(|d| d.details.description.clone())
                .unwrap_or_default(),
            source
                .and_then(|s| s.line.description.clone())
                .unwrap_or_default(),
            source
                .and_then(|s| s.line.set_number.clone())
                .unwrap_or_default(),
            purchase.map(|p| p.draft.notes.clone()).unwrap_or_default(),
            source.map(|s| s.line.notes.clone()).unwrap_or_default(),
        ];
        let archived_reference = definition.is_some_and(|d| d.archived)
            || manufacturer.archived
            || color.archived
            || condition.archived
            || location.iter().any(|r| r.archived);
        let incomplete = definition.is_none()
            || normalized_number(&definition.and_then(|d| d.details.number.clone())).is_none()
            || manufacturer.id == self.unknown_manufacturer
            || condition.id == self.unknown_condition
            || location.is_empty()
            || matches!(target, Target::Holding(_)) && color.id == self.unknown_color
            || matches!(target, Target::UnquantifiedSetLine(_));
        Ok((
            InventoryRow {
                archived: links.archived || purchase.is_some_and(|p| p.archived),
                target,
                kind,
                label,
                number,
                definition_id: links.definition_id,
                manufacturer,
                color,
                condition,
                location,
                assembly: brickxinator_core::inventory::AssemblyState::Loose,
                holding: None,
                set_copy_id: None,
                purchase_id: purchase.map(|p| p.id),
                invoice_line_id: source.map(|s| s.line.id),
                seller: purchase.and_then(|p| p.draft.seller.clone()),
                stated_piece_count: definition.and_then(|d| d.details.stated_piece_count),
                verification: None,
                archived_reference,
                incomplete,
            },
            text.join(" "),
        ))
    }
}

fn rebuild(connection: &Connection) -> Result<()> {
    let ledger = load(connection)?;
    let settings = load_settings(connection)?;
    let definitions = load_catalog(connection, &settings)?;
    let profiles = load_profiles(connection, &settings, &definitions)?;
    let defaults = default_links(&settings)?;
    let ids: Vec<String> = connection
        .prepare("SELECT id FROM purchase")?
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    let mut purchases = BTreeMap::new();
    let mut lines = BTreeMap::new();
    for id in ids {
        let id = parse_id(id)?;
        let purchase = read_purchase(connection, id)?.ok_or("missing purchase")?;
        for line in &purchase.draft.lines {
            lines.insert(
                line.id,
                Source {
                    purchase: id,
                    line: line.clone(),
                },
            );
        }
        purchases.insert(id, purchase);
    }
    let context = Context {
        settings: &settings,
        definitions: &definitions,
        purchases: &purchases,
        lines: &lines,
        unknown_manufacturer: unknown(&settings, Vocabulary::Manufacturer)?,
        unknown_color: defaults.color_id,
        unknown_condition: defaults.condition_id,
    };
    let children: BTreeMap<Id, Id> = ledger
        .operations()
        .iter()
        .filter_map(|op| {
            if let Body::Identify {
                source_id,
                child_id,
                ..
            } = op.body
            {
                Some((child_id, source_id))
            } else {
                None
            }
        })
        .collect();
    let contents: BTreeMap<Id, _> = ledger
        .verification()
        .iter()
        .flat_map(|(copy, state)| {
            state
                .lines
                .values()
                .filter(|line| line.verified.is_some())
                .map(move |line| (line.holding_id, (*copy, line)))
        })
        .collect();
    let mut documents = vec![];
    for (id, holding) in ledger.current_holdings() {
        let source = ledger.provenance().get(&id).copied();
        let mut links = profiles
            .get(&ProfileTarget::Holding(id))
            .map(|p| p.links.clone())
            .unwrap_or_else(|| defaults.clone());
        let mut parent_text = String::new();
        let kind = if let Some((copy, line)) = contents.get(&id) {
            let parent = profiles.get(&ProfileTarget::SetCopy(*copy));
            links = Links {
                archived: parent.is_some_and(|p| p.links.archived),
                definition_id: line.definition_id,
                color_id: line.color_id,
                condition_id: parent.map_or(defaults.condition_id, |p| p.links.condition_id),
                location_id: parent.and_then(|p| p.links.location_id),
            };
            parent_text = ledger.copy_label(*copy).unwrap_or_default().to_owned();
            if let Some(definition) = parent
                .and_then(|p| p.links.definition_id)
                .and_then(|id| definitions.get(&id))
            {
                parent_text.push_str(&format!(
                    " {} {}",
                    definition.details.number.as_deref().unwrap_or(""),
                    definition.details.name.as_deref().unwrap_or("")
                ));
            }
            Kind::SetContent
        } else if children.contains_key(&id) {
            Kind::IdentifiedBatch
        } else {
            match source.and_then(|id| lines.get(&id)).map(|s| s.line.kind) {
                Some(LineKind::MixedLot) => Kind::MixedLot,
                Some(LineKind::LooseBatch) => Kind::LooseBatch,
                _ => Kind::OpeningHolding,
            }
        };
        let (mut row, text) = context.row(Target::Holding(id), kind, &links, source)?;
        row.assembly = holding.assembly;
        row.holding = Some(holding);
        row.set_copy_id = ledger.set_contents().get(&id).copied();
        row.incomplete |= holding.pieces.owned() == Measure::Unknown
            && holding.weight_mg.owned() == Measure::Unknown;
        documents.push((row, format!("{text} {parent_text}")));
    }
    for (id, copy) in ledger.set_copies() {
        let links = profiles
            .get(&ProfileTarget::SetCopy(*id))
            .map(|p| &p.links)
            .unwrap_or(&defaults);
        let (mut row, text) = context.row(
            Target::SetCopy(*id),
            Kind::SetCopy,
            links,
            Some(copy.invoice_line_id),
        )?;
        let title = row.label.clone();
        row.label = ledger.copy_label(copy.id).unwrap_or_default().to_owned();
        row.assembly = ledger.assembly(*id).ok_or("missing assembly")?;
        row.set_copy_id = Some(*id);
        row.verification = Some(
            ledger
                .verification()
                .get(id)
                .map_or(verification::Status::NotStarted, |s| s.status()),
        );
        documents.push((row, format!("{title} {text}")));
    }
    for (id, source) in &lines {
        if purchases[&source.purchase].status == Status::Received
            && source.line.kind == LineKind::Set
            && source.line.copy_count.is_none()
        {
            let (mut row, text) = context.row(
                Target::UnquantifiedSetLine(*id),
                Kind::UnquantifiedSetLine,
                &defaults,
                Some(*id),
            )?;
            row.assembly = source.line.assembly;
            documents.push((row, text));
        }
    }
    connection.execute("DELETE FROM inventory_search_fts", [])?;
    connection.execute("DELETE FROM inventory_search_document", [])?;
    let mut totals = InventoryTotals::default();
    for (row, extra) in documents {
        totals.add(&row);
        let location_ids: Vec<String> = row.location.iter().map(|r| r.id.to_string()).collect();
        let location_names = row
            .location
            .iter()
            .map(|r| r.name.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let text = format!(
            "{} {} {} {} {} {} {} {} {}",
            row.label,
            row.number.as_deref().unwrap_or(""),
            row.manufacturer.name,
            row.color.name,
            row.condition.name,
            location_names,
            row.seller.as_deref().unwrap_or(""),
            row.target.id(),
            extra
        );
        connection.execute("INSERT INTO inventory_search_document(target_kind,target_id,row_kind,normalized_number,manufacturer_id,color_id,condition_id,location_id,location_ids_json,available,assembled,damaged,unknown_measure,incomplete,archived_reference,snapshot_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",params![target_name(row.target),row.target.id().to_string(),kind_name(row.kind),normalized_number(&row.number),row.manufacturer.id.to_string(),row.color.id.to_string(),row.condition.id.to_string(),row.location.last().map(|r|r.id.to_string()),serde_json::to_string(&location_ids)?,row.matches_availability(Availability::Available),row.matches_availability(Availability::InAssembledSet),row.matches_availability(Availability::BrokenDamaged),row.matches_availability(Availability::Unknown),row.incomplete,row.archived_reference,serde_json::to_string(&row)?])?;
        let rowid = connection.last_insert_rowid();
        connection.execute(
            "INSERT INTO inventory_search_fts(rowid,search_text) VALUES(?1,?2)",
            params![rowid, text],
        )?;
    }
    let pending_deliveries = purchases
        .values()
        .filter(|p| matches!(p.status, Status::Ordered | Status::Shipped))
        .count() as u64;
    let incomplete_purchases = purchases
        .values()
        .filter(|p| {
            p.status != Status::Cancelled
                && (p.draft.seller.as_ref().is_none_or(|s| s.trim().is_empty())
                    || p.draft.total_cents().is_none()
                    || p.draft.lines.is_empty()
                    || p.draft.lines.iter().any(|l| {
                        l.kind == LineKind::Set && l.copy_count.is_none()
                            || l.kind != LineKind::Set
                                && l.pieces == Measure::Unknown
                                && l.weight_mg == Measure::Unknown
                    }))
        })
        .count() as u64;
    let mut recent_purchases = vec![];
    let mut query=connection.prepare("SELECT c.purchase_id,c.committed_at FROM purchase_command c JOIN purchase p ON p.id=c.purchase_id WHERE coalesce(json_extract(p.snapshot_json,'$.archived'),0)=0 AND c.sequence IN (SELECT min(sequence) FROM purchase_command GROUP BY purchase_id) ORDER BY c.sequence DESC LIMIT 5")?;
    let mut rows = query.query([])?;
    while let Some(row) = rows.next()? {
        let id = parse_id(row.get(0)?)?;
        let purchase = &purchases[&id];
        recent_purchases.push(PurchaseCard {
            id,
            status: purchase.status,
            seller: purchase.draft.seller.clone(),
            total_cents: purchase.draft.total_cents(),
            recorded_at: row.get(1)?,
        });
    }
    let dashboard = Dashboard {
        inventory: totals,
        pending_deliveries,
        incomplete_purchases,
        recent_purchases,
    };
    connection.execute(
        "INSERT INTO inventory_search_fts(inventory_search_fts) VALUES('integrity-check')",
        [],
    )?;
    connection.execute(
        "UPDATE inventory_search_state SET dirty=0,format_version=2,dashboard_json=?1 WHERE id=1",
        [serde_json::to_string(&dashboard)?],
    )?;
    Ok(())
}
