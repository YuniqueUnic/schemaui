use crate::keymap_spec::validate_unique_ids;

#[test]
fn validate_unique_ids_accepts_unique_inputs() {
    let ids = ["one", "two", "three"];
    validate_unique_ids(ids).expect("unique ids should pass");
}

#[test]
fn validate_unique_ids_rejects_duplicates() {
    let ids = ["dup", "dup"];
    let error = validate_unique_ids(ids).expect_err("duplicate ids should fail");
    assert_eq!(error, "duplicate keymap entry id dup");
}
