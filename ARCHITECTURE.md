# Architecture

## Current Direction
We are following a **speed-first, architecture-safe** approach:

1. Ship MVP features quickly.
2. Continuously refactor into clear layers as features land.
3. Avoid big-bang rewrites.

This gives fast iteration now and maintainability later.

## Target Layered Structure

```text
src/
  api/            # HTTP handlers, routing, request/response DTOs
  application/    # use-cases (incident lifecycle, timeline, assignments)
  domain/         # core models, invariants, business rules
  infra/          # db/repositories, external integrations
  main.rs         # bootstrap + wiring
```

## Current State (2026-03-10)
- Axum HTTP server
- SQLite persistence with sqlx
- Incident lifecycle APIs (create/list/get/ack/resolve)
- Incident event timeline and notes

## Architectural Constraints
- Keep API behavior backward compatible unless versioned.
- Keep domain types explicit (`Severity`, `IncidentStatus`, `EventType`).
- No framework types in domain layer once modular split begins.
- Prefer small, incremental PRs and ADRs for major design choices.

## Next Evolution Steps
1. Modularize `main.rs` into `api`, `application`, `infra`, `domain`.
2. Add repository abstraction (trait) for incident store.
3. Add API key auth middleware.
4. Add pagination/filtering contracts.
5. Add integration tests using ephemeral SQLite DB.
