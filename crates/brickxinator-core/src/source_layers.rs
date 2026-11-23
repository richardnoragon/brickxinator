//! Quantity provenance through physical movements. Ambiguous mixed-source splits
//! retain source identities with unknown quantities rather than assuming FIFO.
use crate::{
    inventory::Measure,
    ledger::{Body, Id, Ledger, LedgerError},
    physical::{AcquisitionPortion, Portion},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layer {
    pub pieces: Option<u64>,
    pub weight_mg: Option<u64>,
}
type Layers = BTreeMap<Id, Layer>;
fn known(m: Measure) -> Option<u64> {
    if let Measure::Known(n) = m {
        Some(n)
    } else {
        None
    }
}
fn from_holding(h: crate::journal::Holding) -> Layer {
    Layer {
        pieces: known(h.pieces.owned()),
        weight_mg: known(h.weight_mg.owned()),
    }
}
fn add(a: Option<u64>, b: Option<u64>) -> Option<u64> {
    a.zip(b).and_then(|(a, b)| a.checked_add(b))
}
pub fn project(ledger: &Ledger) -> BTreeMap<Id, Layers> {
    let mut result = BTreeMap::<Id, Layers>::new();
    let mut snapshots = BTreeMap::<Id, BTreeMap<Id, Layers>>::new();
    let mut previous = BTreeMap::new();
    for (op, outcome) in ledger.operations().iter().zip(ledger.outcomes()) {
        let before = result.clone();
        let correction=match &op.body{Body::SupplierReturnCorrection{dispatch_operation_id}=>Some(*dispatch_operation_id),Body::Lifecycle(e)=>e.corrects,Body::ReceiptCorrection{receipt_operation_id,reverse:false,..}=>ledger.operations().iter().take((outcome.sequence-1) as usize).rev().find(|o|matches!(o.body,Body::ReceiptCorrection{receipt_operation_id:id,reverse:true,..} if id==*receipt_operation_id)).map(|o|o.id),_=>None};
        if let Some(target) = correction
            && let Some(snapshot) = snapshots.get(&target)
        {
            for id in outcome.holdings.keys() {
                result.insert(*id, snapshot.get(id).cloned().unwrap_or_default());
            }
            previous.extend(outcome.holdings.clone());
            continue;
        }
        for (id, h) in &outcome.holdings {
            if let Some(layers) = result.get_mut(id) {
                if layers.len() == 1 {
                    if let Some(layer) = layers.values_mut().next() {
                        *layer = from_holding(*h);
                    }
                } else {
                    for layer in layers.values_mut() {
                        layer.pieces = if h.pieces.owned() == Measure::Known(0) {
                            Some(0)
                        } else if previous
                            .get(id)
                            .is_some_and(|old: &crate::journal::Holding| {
                                old.pieces.owned() == h.pieces.owned()
                            })
                        {
                            layer.pieces
                        } else {
                            None
                        };
                        layer.weight_mg = if h.weight_mg.owned() == Measure::Known(0) {
                            Some(0)
                        } else if previous
                            .get(id)
                            .is_some_and(|old: &crate::journal::Holding| {
                                old.weight_mg.owned() == h.weight_mg.owned()
                            })
                        {
                            layer.weight_mg
                        } else {
                            None
                        };
                    }
                }
            } else if let Some(line) = ledger.provenance().get(id) {
                result.insert(*id, BTreeMap::from([(*line, from_holding(*h))]));
            }
        }
        match &op.body {
            Body::Merge { sources, target_id } => {
                let mut merged = Layers::new();
                for source in sources {
                    for (id, layer) in before.get(&source.holding_id).into_iter().flatten() {
                        let target = merged.entry(*id).or_insert(Layer {
                            pieces: Some(0),
                            weight_mg: Some(0),
                        });
                        target.pieces = add(target.pieces, layer.pieces);
                        target.weight_mg = add(target.weight_mg, layer.weight_mg);
                    }
                }
                result.insert(*target_id, merged);
            }
            Body::Split {
                source_id,
                child_id,
                ..
            }
            | Body::Identify {
                source_id,
                child_id,
                ..
            } => {
                if let Some(layers) = before.get(source_id) {
                    if layers.len() == 1 {
                        let line = *layers.keys().next().expect("one source");
                        result.insert(
                            *child_id,
                            BTreeMap::from([(line, from_holding(outcome.holdings[child_id]))]),
                        );
                    } else {
                        result.insert(
                            *child_id,
                            layers.keys().map(|id| (*id, Layer::default())).collect(),
                        );
                    }
                }
            }
            Body::Lifecycle(event) => {
                for disposition in &event.parts {
                    if let crate::physical::Handling::Custom { child_id, .. } = disposition.handling
                        && let Some(layers) = before.get(&disposition.holding_id)
                    {
                        let child = if layers.len() == 1 {
                            BTreeMap::from([(
                                *layers.keys().next().expect("one source"),
                                from_holding(outcome.holdings[&child_id]),
                            )])
                        } else {
                            layers.keys().map(|id| (*id, Layer::default())).collect()
                        };
                        result.insert(child_id, child);
                    }
                }
            }
            Body::SupplierReturn {
                holding_id,
                sources,
                ..
            } if !sources.is_empty() => {
                if let Some(layers) = before.get(holding_id) {
                    let mut remaining = layers.clone();
                    for source in sources {
                        if let Some(layer) = remaining.get_mut(&source.invoice_line_id) {
                            layer.pieces = layer
                                .pieces
                                .zip(source.pieces)
                                .and_then(|(a, b)| a.checked_sub(b));
                            layer.weight_mg = layer
                                .weight_mg
                                .zip(source.weight_mg)
                                .and_then(|(a, b)| a.checked_sub(b));
                        }
                    }
                    result.insert(*holding_id, remaining);
                }
            }
            _ => {}
        }
        if matches!(
            op.body,
            Body::SupplierReturn { .. }
                | Body::Lifecycle(_)
                | Body::ReceiptCorrection { reverse: true, .. }
        ) {
            snapshots.insert(
                op.id,
                outcome
                    .holdings
                    .keys()
                    .map(|id| (*id, before.get(id).cloned().unwrap_or_default()))
                    .collect(),
            );
        }
        previous.extend(outcome.holdings.clone());
    }
    result
}
pub fn validate_return(
    layers: &Layers,
    portion: Portion,
    sources: &[AcquisitionPortion],
) -> Result<(), LedgerError> {
    let invalid = LedgerError::InvalidStockAction;
    if layers.len() > 1 && sources.is_empty() {
        return Err(invalid(
            "mixed-source return requires explicit acquisition quantities",
        ));
    }
    if sources.is_empty() {
        return Ok(());
    }
    let mut seen = BTreeSet::new();
    let mut pieces = Some(0u64);
    let mut weight = Some(0u64);
    for source in sources {
        if !seen.insert(source.invoice_line_id) {
            return Err(invalid("return acquisition sources must be unique"));
        }
        let layer = layers.get(&source.invoice_line_id).ok_or(invalid(
            "selected acquisition source is not in this holding",
        ))?;
        for (requested, available) in [
            (source.pieces, layer.pieces),
            (source.weight_mg, layer.weight_mg),
        ] {
            if let Some(n) = requested
                && available.is_none_or(|a| n > a)
            {
                return Err(invalid(
                    "return source quantity is unknown or insufficient; reconcile provenance before return",
                ));
            }
        }
        pieces = add(pieces, source.pieces);
        weight = add(weight, source.weight_mg);
    }
    if pieces != portion.pieces || weight != portion.weight_mg {
        return Err(invalid(
            "explicit source quantities must exactly match returned pieces and weight, preserving unknown units",
        ));
    }
    Ok(())
}
