## Summary

<!-- What changed? Keep this short. -->

## Scope

<!-- Name the affected crates, commands, docs, examples, tests, or public surfaces. -->

## Verification

<!-- List the commands run. If a command was skipped, state why. -->

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Tests

<!-- Describe tests added or updated. If tests do not apply, state why. -->

## Documentation Impact

<!-- State whether docs were updated. If docs do not apply, state why. -->

## Compatibility / Breaking Changes

<!-- Note any command, output, storage, declaration, workflow, or documentation-contract changes. Write “None” if none are expected. -->

## Linked Issue

<!-- Link the issue this closes or advances, if any. -->
