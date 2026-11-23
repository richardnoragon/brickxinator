use brickxinator_core::model_import::ldraw;
const REF: &str = "1 4 0 0 0 1 0 0 0 1 0 0 0 1 3001.dat";
#[test]
fn nested_repeated_models_count_inventory_and_inherit_colors() {
    let text = format!(
        "0 FILE root.ldr\n1 2 0 0 0 1 0 0 0 1 0 0 0 1 child.ldr\n1 2 0 0 0 1 0 0 0 1 0 0 0 1 child.ldr\n0 FILE child.ldr\n{}\n",
        REF.replace("1 4 ", "1 16 ")
    );
    let rows = ldraw(&text).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].reference, "3001.dat");
    assert_eq!(rows[0].quantity, 2);
    assert_eq!(rows[0].color, "2");
}
#[test]
fn incomplete_unresolved_and_cyclic_models_do_not_silently_drop_parts() {
    assert!(ldraw("1 4 0").is_err());
    assert!(ldraw(&REF.replace("3001.dat", "missing.ldr")).is_err());
    let cycle = format!("0 FILE root.ldr\n{}", REF.replace("3001.dat", "root.ldr"));
    assert!(ldraw(&cycle).is_err());
    assert!(ldraw("0 Just a comment").is_err());
}
