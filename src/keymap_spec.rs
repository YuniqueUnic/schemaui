use std::collections::HashSet;

pub(crate) fn validate_unique_ids<I, S>(ids: I) -> Result<(), String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut seen = HashSet::new();
    for id in ids {
        let id = id.as_ref();
        if !seen.insert(id.to_owned()) {
            return Err(format!("duplicate keymap entry id {id}"));
        }
    }
    Ok(())
}
