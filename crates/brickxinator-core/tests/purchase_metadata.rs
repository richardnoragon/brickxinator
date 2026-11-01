use brickxinator_core::purchases::{Draft, TaxPresentation, valid_date};
#[test]
fn dates_obey_gregorian_calendar_and_tax_is_informational() {
    for date in ["2024-02-29", "2000-02-29", "2026-11-01"] {
        assert!(valid_date(date));
    }
    for date in [
        "1900-02-29",
        "2026-02-29",
        "2026-04-31",
        "2026-00-01",
        "0000-01-01",
        "2026-1-01",
        "é026-01-01",
    ] {
        assert!(!valid_date(date));
    }
    let draft = Draft {
        purchased_on: Some("2026-11-01".into()),
        tax_presentation: Some(TaxPresentation::AddedSeparately),
        tax_cents: Some(1900),
        shipping_cents: Some(500),
        ..Default::default()
    };
    draft.validate().unwrap();
    assert_eq!(draft.total_cents(), Some(500));
    // Legacy serialized purchase drafts retain their unknown optional metadata.
    let legacy = r#"{"seller":null,"order_number":null,"source_url":null,"notes":"","shipping_cents":null,"lines":[]}"#;
    let decoded: Draft = serde_json::from_str(legacy).unwrap();
    assert_eq!(decoded, Draft::default());
    assert_eq!(serde_json::to_string(&decoded).unwrap(), legacy);
}
