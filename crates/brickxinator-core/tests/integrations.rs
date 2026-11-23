use brickxinator_core::integrations::*;
#[test]
fn delimited_files_preserve_quoted_fields_and_reject_ambiguous_or_malformed_rows() {
    let t = delimited(
        "part_num,name\r\n3001,\"Brick, tall\"\r\n3002,\"Name with \"\"quotes\"\"\"",
        ',',
    )
    .unwrap();
    assert_eq!(t.rows[0][1], "Brick, tall");
    assert_eq!(t.rows[1][1], "Name with \"quotes\"");
    assert!(delimited("id,id\n1,2", ',').is_err());
    assert!(delimited("id,name\n1", ',').is_err());
    assert!(delimited("id,name\n1,\"unfinished", ',').is_err());
}
#[test]
fn bulk_inventory_requires_an_explicit_inventory_and_spare_policy() {
    let t=delimited("inventory_id,part_num,color_id,quantity,is_spare\n1,3001,0,2,f\n1,3001,0,1,t\n2,3002,4,99,f",',').unwrap();
    let mut c = Columns {
        part: "part_num".into(),
        color: "color_id".into(),
        quantity: "quantity".into(),
        inventory: Some("inventory_id".into()),
        inventory_id: None,
        spare: Some("is_spare".into()),
        include_spares: false,
    };
    assert!(bom(&t, &c).is_err());
    c.inventory_id = Some("1".into());
    let parts = bom(&t, &c).unwrap();
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].quantity, 2);
    c.include_spares = true;
    assert_eq!(bom(&t, &c).unwrap()[0].quantity, 3);
}
#[test]
fn wanted_lists_escape_xml_and_aggregate_exact_external_ids() {
    let xml = wanted_xml(&[
        Wanted {
            part_id: "a&b".into(),
            color_id: 0,
            quantity: 2,
        },
        Wanted {
            part_id: "a&b".into(),
            color_id: 0,
            quantity: 3,
        },
    ])
    .unwrap();
    assert!(xml.contains("<ITEMID>a&amp;b</ITEMID>"));
    assert!(xml.contains("<COLOR>0</COLOR><MINQTY>5</MINQTY>"));
    assert!(
        wanted_xml(&[Wanted {
            part_id: "".into(),
            color_id: 1,
            quantity: 1
        }])
        .is_err()
    );
}
