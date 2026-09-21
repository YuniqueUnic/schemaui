//! Drafts: where they go, what survives, and what stops surviving.

use std::path::{Path, PathBuf};

use serde_json::json;

use crate::draft::{DraftStore, StateDirLayout, state_dir_in};

/// A directory this test owns outright, removed when it drops.
struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock is after the epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("schemaui-{label}-{unique}"));
        std::fs::create_dir_all(&path).expect("create temp dir");
        Self(path)
    }

    fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn every_platform_puts_drafts_where_that_platform_keeps_state() {
    // Testable on any host precisely because the layout is a parameter: the
    // Windows and Linux rules are otherwise only ever exercised by whoever
    // happens to run CI on them.
    let home = Path::new("/home/ada");
    let xdg = Path::new("/home/ada/.state");
    let local_app_data = Path::new("C:\\Users\\Ada\\AppData\\Local");

    assert_eq!(
        state_dir_in(StateDirLayout::MacOs, Some(home), None, None),
        Some(PathBuf::from(
            "/home/ada/Library/Application Support/schemaui"
        ))
    );
    assert_eq!(
        state_dir_in(StateDirLayout::Xdg, Some(home), None, None),
        Some(PathBuf::from("/home/ada/.local/state/schemaui"))
    );
    assert_eq!(
        state_dir_in(StateDirLayout::Xdg, Some(home), Some(xdg), None),
        Some(PathBuf::from("/home/ada/.state/schemaui")),
        "XDG_STATE_HOME outranks the default, which is the point of setting it"
    );
    assert_eq!(
        state_dir_in(
            StateDirLayout::Windows,
            Some(home),
            None,
            Some(local_app_data)
        ),
        Some(PathBuf::from("C:\\Users\\Ada\\AppData\\Local").join("schemaui"))
    );
}

#[test]
fn a_platform_that_offers_nowhere_gets_no_draft_rather_than_a_guess() {
    assert_eq!(state_dir_in(StateDirLayout::Xdg, None, None, None), None);
    assert_eq!(
        state_dir_in(StateDirLayout::Windows, None, None, None),
        None
    );
    assert_eq!(state_dir_in(StateDirLayout::MacOs, None, None, None), None);
}

#[test]
fn a_saved_draft_comes_back() {
    let dir = TempDir::new("draft-roundtrip");
    let store = DraftStore::at(dir.join("nested/session.json"));
    assert_eq!(store.load(), None, "nothing saved yet");

    let value = json!({ "name": "ada", "port": 8080 });
    store
        .save(&value)
        .expect("save should create its directory");

    assert_eq!(store.load(), Some(value));
}

#[test]
fn saving_twice_keeps_the_second_answer() {
    let dir = TempDir::new("draft-overwrite");
    let store = DraftStore::at(dir.join("session.json"));

    store.save(&json!({ "step": 1 })).expect("first save");
    store.save(&json!({ "step": 2 })).expect("second save");

    assert_eq!(store.load(), Some(json!({ "step": 2 })));
    assert!(
        !dir.join("session.json.part").exists(),
        "the staging file must not survive: a stray half-written draft is the \
         exact failure this write path exists to avoid"
    );
}

#[test]
fn a_discarded_draft_is_gone_and_discarding_twice_is_harmless() {
    let dir = TempDir::new("draft-discard");
    let store = DraftStore::at(dir.join("session.json"));
    store.save(&json!({ "answer": 42 })).expect("save");

    store.discard();
    assert_eq!(store.load(), None);

    // Reached whenever a session ends twice over — an exit racing a deadline,
    // say — and must not turn a finished session into a failed one.
    store.discard();
    assert_eq!(store.load(), None);
}

#[test]
fn an_unreadable_draft_is_skipped_rather_than_fatal() {
    let dir = TempDir::new("draft-corrupt");
    let path = dir.join("session.json");
    std::fs::write(&path, "{ this is not json").expect("write corrupt draft");

    // Losing a draft is bad; refusing to open the form because of one is worse.
    assert_eq!(DraftStore::at(&path).load(), None);
}

#[cfg(unix)]
#[test]
fn drafts_are_readable_only_by_their_owner() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new("draft-perms");
    let store = DraftStore::at(dir.join("session.json"));
    store
        .save(&json!({ "api_key": "s3cret" }))
        .expect("save should succeed");

    let mode = std::fs::metadata(store.path())
        .expect("draft metadata")
        .permissions()
        .mode();
    assert_eq!(
        mode & 0o777,
        0o600,
        "drafts hold whatever the user typed, passwords included"
    );
}

#[test]
fn the_same_form_resolves_to_the_same_draft_and_a_different_one_does_not() {
    let schema = json!({ "type": "object", "properties": { "port": { "type": "integer" } } });
    let reordered = json!({ "properties": { "port": { "type": "integer" } }, "type": "object" });
    let other = json!({ "type": "object", "properties": { "host": { "type": "string" } } });

    let Ok(first) = DraftStore::for_schema(&schema) else {
        // A machine with no home directory runs without drafts by design.
        return;
    };
    let second = DraftStore::for_schema(&reordered).expect("state dir resolved once already");
    let different = DraftStore::for_schema(&other).expect("state dir resolved once already");

    assert_eq!(
        first.path(),
        second.path(),
        "key order is not identity: the same form written two ways must find \
         its own draft"
    );
    assert_ne!(first.path(), different.path());
}
