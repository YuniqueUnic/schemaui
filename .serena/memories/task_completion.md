# Task Completion Checklist

1. Run `cargo fmt` and ensure no formatting drift.
2. Run `cargo test` (or targeted sub-crate tests) and confirm all pass.
3. Update relevant docs (`docs/en/progress.md`, `docs/en/todo_manager.md`,
   design notes) describing the change.
4. Record outstanding follow-ups in TodoManager/docs if scope spills over.
5. Review `git status -sb` to verify only intentional files changed before
   handing work off.
