//! Versioned build requirements and quantity claims. No command creates ownership.
use crate::ledger::Id;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
type Result<T> = std::result::Result<T, String>;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    OfficialSetBuild,
    Moc,
    TrainLayout,
    Diorama,
    Custom,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Progress {
    Planned,
    InProgress,
    Completed,
    Dismantled,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClaimKind {
    Reserved,
    Allocated,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnitKind {
    Bag,
    Subassembly,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnitProgress {
    Unopened,
    Opened,
    NotStarted,
    InProgress,
    Completed,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Requirement {
    pub id: Id,
    pub definition_id: Id,
    pub color_id: Id,
    pub quantity: u64,
    pub unit_estimate_cents: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_snapshot_id: Option<Id>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Version {
    pub id: Id,
    pub design_id: Id,
    pub label: String,
    pub author: String,
    pub notes: String,
    pub source: String,
    pub requirements: Vec<Requirement>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: Id,
    pub name: String,
    pub kind: Kind,
    pub version_id: Id,
    pub copy_id: Option<Id>,
    pub progress: Progress,
    pub notes: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unit {
    pub id: Id,
    pub project_id: Id,
    pub label: String,
    pub kind: UnitKind,
    pub progress: UnitProgress,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claim {
    pub id: Id,
    pub project_id: Id,
    pub requirement_id: Id,
    pub holding_id: Id,
    pub quantity: u64,
    pub kind: ClaimKind,
    pub verified_revision: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stock {
    pub revision: u64,
    pub owned: Option<u64>,
    pub damaged: Option<u64>,
    pub definition_id: Option<Id>,
    pub color_id: Id,
    // Existing copy assignments are protected until explicitly detached.
    pub copy_id: Option<Id>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingEvidence {
    pub project_id: Id,
    pub requirement_id: Id,
    pub quantity: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriceSnapshot {
    pub id: Id,
    pub definition_id: Id,
    pub source: String,
    pub observed_on: String,
    pub condition: String,
    pub currency: String,
    pub value_cents: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalMapping {
    pub definition_id: Id,
    pub color_id: Id,
    pub bricklink_part_id: String,
    pub bricklink_color_id: u32,
    pub notes: String,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub revision: u64,
    pub versions: BTreeMap<Id, Version>,
    pub projects: BTreeMap<Id, Project>,
    pub claims: BTreeMap<Id, Claim>,
    pub used_claim_ids: std::collections::BTreeSet<Id>,
    pub units: BTreeMap<Id, Unit>,
    pub confirmed_missing: BTreeMap<Id, MissingEvidence>,
    pub price_snapshots: BTreeMap<Id, PriceSnapshot>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub external_mappings: BTreeMap<String, ExternalMapping>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub id: Id,
    pub request_key: String,
    pub actor: String,
    pub reason: String,
    pub expected_revision: u64,
    pub action: Action,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    ConfirmExternalMapping(ExternalMapping),
    AddPriceSnapshot(PriceSnapshot),
    ConfirmMissing {
        project_id: Id,
        requirement_id: Id,
        quantity: u64,
    },
    AddVersion(Version),
    Create(Project),
    ChangeVersion {
        project_id: Id,
        version_id: Id,
    },
    Claim(Claim),
    Release {
        claim_id: Id,
        #[serde(default)]
        quantity: Option<u64>,
    },
    Transfer {
        claim_id: Id,
        project_id: Id,
        requirement_id: Id,
        #[serde(default)]
        quantity: Option<u64>,
        #[serde(default)]
        new_claim_id: Option<Id>,
    },
    Verify {
        claim_id: Id,
        holding_revision: u64,
    },
    Start {
        project_id: Id,
    },
    Progress {
        project_id: Id,
        progress: Progress,
    },
    Unit(Unit),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Readiness {
    Planned,
    Blocked,
    PotentiallyReady,
    ReadyToBuild,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Coverage {
    pub requirement_id: Id,
    pub required: u64,
    pub allocated: u64,
    pub reserved: u64,
    pub verified: u64,
    pub potential_gap: u64,
    pub confirmed_missing: u64,
    pub unknown_verification: u64,
    pub estimate_cents: Option<crate::finance::Exact>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcurementNeed {
    pub definition_id: Id,
    pub color_id: Id,
    pub confirmed_missing: crate::finance::Exact,
    pub potential_gap: crate::finance::Exact,
    pub unknown_verification: crate::finance::Exact,
    pub projects: Vec<Id>,
}
impl State {
    pub fn procurement(&self, stock: &BTreeMap<Id, Stock>) -> Result<Vec<ProcurementNeed>> {
        let mut needs = BTreeMap::<(Id, Id), ProcurementNeed>::new();
        for p in self.projects.values() {
            for row in self.coverage(p.id, stock)? {
                let r = self.versions[&p.version_id]
                    .requirements
                    .iter()
                    .find(|r| r.id == row.requirement_id)
                    .ok_or("requirement missing")?;
                let n = needs
                    .entry((r.definition_id, r.color_id))
                    .or_insert(ProcurementNeed {
                        definition_id: r.definition_id,
                        color_id: r.color_id,
                        confirmed_missing: crate::finance::Exact(0),
                        potential_gap: crate::finance::Exact(0),
                        unknown_verification: crate::finance::Exact(0),
                        projects: vec![],
                    });
                n.confirmed_missing.0 += u128::from(row.confirmed_missing);
                n.potential_gap.0 += u128::from(row.potential_gap);
                n.unknown_verification.0 += u128::from(row.unknown_verification);
                n.projects.push(p.id);
            }
        }
        Ok(needs.into_values().collect())
    }
    pub fn apply(&self, command: &Command, stock: &BTreeMap<Id, Stock>) -> Result<Self> {
        if command.expected_revision != self.revision {
            return Err("project revision changed since preview".into());
        }
        if command.actor.trim().is_empty()
            || command.reason.trim().is_empty()
            || command.request_key.trim().is_empty()
        {
            return Err("actor, reason and request key are required".into());
        }
        let mut next = self.clone();
        match &command.action {
            Action::ConfirmExternalMapping(mapping) => {
                if mapping.bricklink_part_id.trim().is_empty()
                    || mapping.bricklink_part_id.len() > 128
                    || mapping.bricklink_part_id.chars().any(char::is_control)
                {
                    return Err("enter an explicit external part identifier".into());
                }
                next.external_mappings.insert(
                    format!("{}:{}", mapping.definition_id, mapping.color_id),
                    mapping.clone(),
                );
            }
            Action::AddPriceSnapshot(quote) => {
                if next.price_snapshots.contains_key(&quote.id)
                    || quote.source.trim().is_empty()
                    || quote.condition.trim().is_empty()
                    || !crate::purchases::valid_date(&quote.observed_on)
                    || quote.currency.len() != 3
                    || !quote.currency.chars().all(|c| c.is_ascii_uppercase())
                {
                    return Err("invalid or duplicate price snapshot; provide source, date, currency and condition".into());
                }
                next.price_snapshots.insert(quote.id, quote.clone());
            }
            Action::ConfirmMissing {
                project_id,
                requirement_id,
                quantity,
            } => {
                let p = next
                    .projects
                    .get(project_id)
                    .ok_or("project does not exist")?;
                let r = next.versions[&p.version_id]
                    .requirements
                    .iter()
                    .find(|r| r.id == *requirement_id)
                    .ok_or("requirement does not exist")?;
                let covered: u64 = next
                    .claims
                    .values()
                    .filter(|c| c.project_id == *project_id && c.requirement_id == *requirement_id)
                    .map(|c| c.quantity)
                    .sum();
                if *quantity > r.quantity.saturating_sub(covered) {
                    return Err("confirmed shortage exceeds uncovered requirement".into());
                }
                next.confirmed_missing.retain(|_, e| {
                    e.project_id != *project_id || e.requirement_id != *requirement_id
                });
                if *quantity > 0 {
                    next.confirmed_missing.insert(
                        command.id,
                        MissingEvidence {
                            project_id: *project_id,
                            requirement_id: *requirement_id,
                            quantity: *quantity,
                        },
                    );
                }
            }
            Action::AddVersion(version) => {
                if next.versions.contains_key(&version.id)
                    || version.label.trim().is_empty()
                    || version.requirements.len() > 10_000
                {
                    return Err("version identity exists or invalid requirements".into());
                }
                let mut ids = std::collections::BTreeSet::new();
                let mut keys = std::collections::BTreeSet::new();
                for r in &version.requirements {
                    if let Some(id) = r.price_snapshot_id {
                        let quote = next
                            .price_snapshots
                            .get(&id)
                            .ok_or("price snapshot does not exist")?;
                        if quote.definition_id != r.definition_id
                            || quote.currency != "EUR"
                            || Some(quote.value_cents) != r.unit_estimate_cents
                        {
                            return Err(
                                "estimate must match the selected EUR price observation and part"
                                    .into(),
                            );
                        }
                    }
                    if r.quantity == 0
                        || !ids.insert(r.id)
                        || !keys.insert((r.definition_id, r.color_id))
                    {
                        return Err("requirements need unique part/color identities and positive quantities".into());
                    }
                }
                next.versions.insert(version.id, version.clone());
            }
            Action::Create(project) => {
                if next.projects.contains_key(&project.id)
                    || project.name.trim().is_empty()
                    || project.progress != Progress::Planned
                    || !next.versions.contains_key(&project.version_id)
                {
                    return Err("invalid new project or version".into());
                }
                if project.copy_id.is_some() && project.kind != Kind::OfficialSetBuild {
                    return Err("only official-set projects link a set copy".into());
                }
                if project
                    .copy_id
                    .is_some_and(|id| next.projects.values().any(|p| p.copy_id == Some(id)))
                {
                    return Err("copy already linked to a project".into());
                }
                next.projects.insert(project.id, project.clone());
            }
            Action::ChangeVersion {
                project_id,
                version_id,
            } => {
                if !next.versions.contains_key(version_id) {
                    return Err("version does not exist".into());
                }
                if next.claims.values().any(|c| c.project_id == *project_id) {
                    return Err(
                        "release or transfer affected claims before changing version".into(),
                    );
                }
                next.projects
                    .get_mut(project_id)
                    .ok_or("project does not exist")?
                    .version_id = *version_id;
            }
            Action::Claim(claim) => {
                if claim.quantity == 0
                    || claim.verified_revision.is_some()
                    || next.used_claim_ids.contains(&claim.id)
                {
                    return Err("invalid new claim; verification is an explicit event".into());
                }
                next.used_claim_ids.insert(claim.id);
                next.claims.insert(claim.id, claim.clone());
            }
            Action::Release { claim_id, quantity } => {
                let old = next
                    .claims
                    .get(claim_id)
                    .ok_or("claim does not exist")?
                    .quantity;
                let amount = quantity.unwrap_or(old);
                if amount == 0 || amount > old {
                    return Err("release quantity exceeds claim".into());
                }
                if amount == old {
                    next.claims.remove(claim_id);
                } else {
                    next.claims
                        .get_mut(claim_id)
                        .ok_or("claim does not exist")?
                        .quantity -= amount;
                }
            }
            Action::Transfer {
                claim_id,
                project_id,
                requirement_id,
                quantity,
                new_claim_id,
            } => {
                let mut moved = next
                    .claims
                    .get(claim_id)
                    .ok_or("claim does not exist")?
                    .clone();
                let amount = quantity.unwrap_or(moved.quantity);
                if amount == 0 || amount > moved.quantity {
                    return Err("transfer quantity exceeds claim".into());
                }
                if amount < moved.quantity {
                    let id = new_claim_id.ok_or("partial transfer needs a fresh claim identity")?;
                    if next.used_claim_ids.contains(&id) {
                        return Err("claim identity already exists".into());
                    }
                    next.claims
                        .get_mut(claim_id)
                        .ok_or("claim does not exist")?
                        .quantity -= amount;
                    next.used_claim_ids.insert(id);
                    moved.id = id;
                } else {
                    next.claims.remove(claim_id);
                }
                moved.project_id = *project_id;
                moved.requirement_id = *requirement_id;
                moved.quantity = amount;
                moved.verified_revision = None;
                next.claims.insert(moved.id, moved);
            }
            Action::Verify {
                claim_id,
                holding_revision,
            } => {
                let c = next
                    .claims
                    .get_mut(claim_id)
                    .ok_or("claim does not exist")?;
                if stock
                    .get(&c.holding_id)
                    .ok_or("holding does not exist")?
                    .revision
                    != *holding_revision
                {
                    return Err("holding revision changed since physical verification".into());
                }
                c.verified_revision = Some(*holding_revision);
            }
            Action::Start { project_id } => {
                if self.readiness(*project_id, stock)? != Readiness::ReadyToBuild {
                    return Err("build requires verified usable committed coverage".into());
                }
                for c in next
                    .claims
                    .values_mut()
                    .filter(|c| c.project_id == *project_id)
                {
                    c.kind = ClaimKind::Allocated;
                }
                next.projects
                    .get_mut(project_id)
                    .ok_or("project does not exist")?
                    .progress = Progress::InProgress;
            }
            Action::Progress {
                project_id,
                progress,
            } => {
                if *progress == Progress::InProgress {
                    return Err("use Start Build to convert reservations explicitly".into());
                }
                next.projects
                    .get_mut(project_id)
                    .ok_or("project does not exist")?
                    .progress = progress.clone();
            }
            Action::Unit(unit) => {
                if !next.projects.contains_key(&unit.project_id) || unit.label.trim().is_empty() {
                    return Err("invalid build unit".into());
                }
                if let Some(old) = next.units.get(&unit.id)
                    && (old.kind != unit.kind || old.project_id != unit.project_id)
                {
                    return Err("unit identity/type cannot change".into());
                }
                let valid = match unit.kind {
                    UnitKind::Bag => matches!(
                        unit.progress,
                        UnitProgress::Unopened | UnitProgress::Opened | UnitProgress::Completed
                    ),
                    UnitKind::Subassembly => matches!(
                        unit.progress,
                        UnitProgress::NotStarted
                            | UnitProgress::InProgress
                            | UnitProgress::Completed
                    ),
                };
                if !valid {
                    return Err("progress does not match build-unit type".into());
                }
                next.units.insert(unit.id, unit.clone());
            }
        }
        let mut affected = vec![];
        match &command.action {
            Action::Claim(c) => affected.push(c.project_id),
            Action::Release { claim_id, .. } => {
                if let Some(c) = self.claims.get(claim_id) {
                    affected.push(c.project_id);
                }
            }
            Action::Transfer {
                claim_id,
                project_id,
                ..
            } => {
                affected.push(*project_id);
                if let Some(c) = self.claims.get(claim_id) {
                    affected.push(c.project_id);
                }
            }
            Action::ChangeVersion { project_id, .. } => affected.push(*project_id),
            _ => {}
        }
        next.confirmed_missing
            .retain(|_, e| !affected.contains(&e.project_id));
        next.validate(stock)?;
        next.revision = next.revision.checked_add(1).ok_or("revision overflow")?;
        Ok(next)
    }
    pub fn validate(&self, stock: &BTreeMap<Id, Stock>) -> Result<()> {
        let mut sums = BTreeMap::<Id, u64>::new();
        let mut requirement_sums = BTreeMap::<(Id, Id), u64>::new();
        for c in self.claims.values() {
            let p = self
                .projects
                .get(&c.project_id)
                .ok_or("claim project does not exist")?;
            let r = self
                .versions
                .get(&p.version_id)
                .and_then(|v| v.requirements.iter().find(|r| r.id == c.requirement_id))
                .ok_or("claim requirement does not exist")?;
            let h = stock
                .get(&c.holding_id)
                .ok_or("claim holding does not exist")?;
            if h.definition_id != Some(r.definition_id) || h.color_id != r.color_id {
                return Err("claim part/color does not match requirement".into());
            }
            if h.copy_id.is_some() {
                return Err("detach existing set contents before claiming them; current copy assignment is protected".into());
            }
            let usable = h
                .owned
                .zip(h.damaged)
                .and_then(|(o, d)| o.checked_sub(d))
                .ok_or("claim needs known usable piece quantity")?;
            let total = sums.entry(c.holding_id).or_default();
            *total = total.checked_add(c.quantity).ok_or("quantity overflow")?;
            if *total > usable {
                return Err("active reservations/allocations exceed usable stock; release or resolve claims first".into());
            }
            let required = requirement_sums
                .entry((c.project_id, c.requirement_id))
                .or_default();
            *required = required
                .checked_add(c.quantity)
                .ok_or("requirement sum overflow")?;
            if *required > r.quantity {
                return Err(
                    "claim exceeds this requirement; review the BOM before adding excess".into(),
                );
            }
        }
        Ok(())
    }
    pub fn claimed(&self, holding_id: Id) -> u64 {
        self.claims
            .values()
            .filter(|c| c.holding_id == holding_id)
            .map(|c| c.quantity)
            .sum()
    }
    pub fn coverage(&self, project_id: Id, stock: &BTreeMap<Id, Stock>) -> Result<Vec<Coverage>> {
        let p = self
            .projects
            .get(&project_id)
            .ok_or("project does not exist")?;
        let v = self
            .versions
            .get(&p.version_id)
            .ok_or("version does not exist")?;
        Ok(v.requirements
            .iter()
            .map(|r| {
                let mut allocated = 0;
                let mut reserved = 0;
                let mut verified = 0;
                for c in self
                    .claims
                    .values()
                    .filter(|c| c.project_id == project_id && c.requirement_id == r.id)
                {
                    match c.kind {
                        ClaimKind::Reserved => reserved += c.quantity,
                        ClaimKind::Allocated => allocated += c.quantity,
                    }
                    if c.verified_revision.is_some()
                        && stock
                            .get(&c.holding_id)
                            .is_some_and(|h| Some(h.revision) == c.verified_revision)
                    {
                        verified += c.quantity;
                    }
                }
                let gap = r.quantity.saturating_sub(allocated + reserved);
                let confirmed_missing = self
                    .confirmed_missing
                    .values()
                    .filter(|e| e.project_id == project_id && e.requirement_id == r.id)
                    .map(|e| e.quantity)
                    .max()
                    .unwrap_or(0)
                    .min(gap);
                Coverage {
                    requirement_id: r.id,
                    required: r.quantity,
                    allocated,
                    reserved,
                    verified,
                    potential_gap: gap - confirmed_missing,
                    confirmed_missing,
                    unknown_verification: (allocated + reserved).saturating_sub(verified),
                    estimate_cents: r
                        .unit_estimate_cents
                        .map(|n| crate::finance::Exact(u128::from(n) * u128::from(gap))),
                }
            })
            .collect())
    }
    pub fn readiness(&self, project_id: Id, stock: &BTreeMap<Id, Stock>) -> Result<Readiness> {
        let rows = self.coverage(project_id, stock)?;
        if rows.is_empty() {
            return Ok(Readiness::Planned);
        }
        if rows.iter().any(|r| r.confirmed_missing > 0) {
            return Ok(Readiness::Blocked);
        }
        // A planning gap is not confirmed physically missing.
        if rows.iter().any(|r| r.potential_gap > 0) {
            return Ok(Readiness::Planned);
        }
        if rows.iter().any(|r| r.verified < r.required) {
            return Ok(Readiness::PotentiallyReady);
        }
        Ok(Readiness::ReadyToBuild)
    }
}
