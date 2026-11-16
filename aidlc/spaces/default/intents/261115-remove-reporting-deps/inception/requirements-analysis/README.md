# Remove reporting-domain serialization dependencies

This intent removes unused `serde` and `serde_json` dependencies from `reporting-domain` to restore the enforced Clean Architecture boundary.

The change is implemented and the code review is READY. Validation passed: `cargo check -p reporting-domain`, `cargo test -p reporting-domain` (46 tests), and `bash scripts/check-boundaries.sh` (60 packages).

See `requirements.md`, `../construction/code-generation/code-summary.md`, and `../construction/build-and-test/test-results.md` for scope, implementation and verification evidence.