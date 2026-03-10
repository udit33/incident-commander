# ADR 0001: Staged Layered Architecture (Speed-first + Guardrails)

- **Status:** Accepted
- **Date:** 2026-03-10
- **Decision Makers:** Project maintainers

## Context
The project needs fast MVP delivery and long-term maintainability for open-source collaboration.
A full architecture rewrite up-front would slow delivery; ad-hoc coding would create future debt.

## Decision
Adopt a staged architecture approach:
- Build features quickly in current codebase.
- Refactor each feature into layers as it stabilizes.
- Converge on domain/application/infra/api boundaries incrementally.

## Consequences
### Positive
- Fast feature velocity early.
- Lower risk than big-bang architecture migration.
- Easier contributor onboarding with gradually improving structure.

### Negative
- Temporary mixed architecture during transition.
- Requires discipline to refactor continuously.

## Follow-up Work
- Split `main.rs` modules in next development cycle.
- Add repository trait boundary before adding external integrations.
- Document each major architectural decision as ADR.
