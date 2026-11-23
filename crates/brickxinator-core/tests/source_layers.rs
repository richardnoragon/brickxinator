use brickxinator_core::{
    inventory::Measure,
    ledger::{Body, Id, Ledger, Operation, ReceivedLine, ReceivedStock},
    physical::{AcquisitionPortion, Portion, Source},
    source_layers,
};
fn append(l: &mut Ledger, body: Body) -> Id {
    let id = Id::new();
    l.append(Operation {
        id,
        request_key: Id::new().to_string(),
        actor: "test".into(),
        reason: "physical source audit".into(),
        body,
    })
    .unwrap();
    id
}
#[test]
fn merged_returns_require_exact_source_quantities_and_safe_correction_restores_them() {
    let mut l = Ledger::default();
    let a = Id::new();
    let b = Id::new();
    let h1 = Id::new();
    let h2 = Id::new();
    for (line, holding, n) in [(a, h1, 100), (b, h2, 50)] {
        append(
            &mut l,
            Body::Receive {
                purchase_id: Id::new(),
                lines: vec![ReceivedLine {
                    invoice_line_id: line,
                    stock: ReceivedStock::Batch {
                        holding_id: holding,
                        pieces: Measure::Known(n),
                        weight_mg: Measure::Unknown,
                    },
                }],
            },
        );
    }
    let merged = Id::new();
    let sources = vec![
        Source {
            holding_id: h1,
            expected_revision: l.holdings()[&h1].revision,
        },
        Source {
            holding_id: h2,
            expected_revision: l.holdings()[&h2].revision,
        },
    ];
    append(
        &mut l,
        Body::Merge {
            sources,
            target_id: merged,
        },
    );
    let portion = Portion {
        pieces: Some(20),
        weight_mg: None,
        damaged_pieces: Some(0),
        damaged_weight_mg: None,
    };
    let body = Body::SupplierReturn {
        holding_id: merged,
        expected_revision: l.holdings()[&merged].revision,
        portion,
        effective_on: "2026-11-23".into(),
        sources: vec![],
    };
    assert!(
        l.append(Operation {
            id: Id::new(),
            request_key: Id::new().to_string(),
            actor: "test".into(),
            reason: "ambiguous return".into(),
            body
        })
        .is_err()
    );
    let rev = l.holdings()[&merged].revision;
    let dispatch = append(
        &mut l,
        Body::SupplierReturn {
            holding_id: merged,
            expected_revision: rev,
            portion,
            effective_on: "2026-11-23".into(),
            sources: vec![AcquisitionPortion {
                invoice_line_id: a,
                pieces: Some(20),
                weight_mg: None,
            }],
        },
    );
    let layers = source_layers::project(&l);
    assert_eq!(layers[&merged][&a].pieces, Some(80));
    assert_eq!(layers[&merged][&b].pieces, Some(50));
    append(
        &mut l,
        Body::SupplierReturnCorrection {
            dispatch_operation_id: dispatch,
        },
    );
    let layers = source_layers::project(&l);
    assert_eq!(layers[&merged][&a].pieces, Some(100));
    assert_eq!(layers[&merged][&b].pieces, Some(50));
}
