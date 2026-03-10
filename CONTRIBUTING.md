# Contributing

Thanks for contributing to Incident Commander.

## Development Setup
1. Install Rust stable (`rustup`)
2. Run locally:
   ```bash
   cargo run
   ```
3. Run checks before opening a PR:
   ```bash
   cargo fmt
   cargo clippy -- -D warnings
   cargo test
   ```

## Contribution Flow
1. Fork the repo and create a feature branch.
2. Keep changes small and focused.
3. Update docs when behavior changes.
4. Add tests for new features or bug fixes.
5. Open a PR with clear description and rationale.

## Architecture Expectations
- Prefer incremental refactors over large rewrites.
- Keep business rules explicit and testable.
- Add/Update ADRs for major architecture decisions (`docs/adr/*`).

## Commit Message Convention
Use conventional commits where possible:
- `feat:` new behavior
- `fix:` bug fixes
- `refactor:` internal restructuring
- `docs:` documentation only
- `test:` tests only
