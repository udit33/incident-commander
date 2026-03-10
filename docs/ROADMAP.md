# Roadmap

## Milestone 0: Bootstrap (Done)
- [x] Rust + Axum server
- [x] Incident CRUD-lite lifecycle
- [x] Timeline events
- [x] SQLite persistence

## Milestone 1: Foundation Hardening (In Progress)
- [ ] Split monolith `main.rs` into layered modules
- [ ] API key auth middleware
- [ ] Pagination + filtering for incident list
- [ ] Integration tests (HTTP + SQLite)
- [ ] OpenAPI spec generation

## Milestone 2: Incident Operations
- [ ] Assignees/on-call ownership
- [ ] State transition rules and policy checks
- [ ] Search endpoints
- [ ] Tagging and priority scoring

## Milestone 3: Integrations
- [ ] Webhook ingestion
- [ ] Slack/Telegram bridge
- [ ] Audit/event export
- [ ] Metrics + tracing instrumentation
