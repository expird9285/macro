# ADR-0000: Title (short, imperative)

- **Status:** Proposed | Accepted | Superseded by ADR-XXXX
- **Date:** YYYY-MM-DD
- **Zone:** RED | YELLOW (per docs/customization-policy.md)
- **Area:** path(s) or subsystem affected

## Context

What task or constraint forced a decision. Why the GREEN/YELLOW extension
points did not cover it. What was investigated (files, consumers,
generation workflows) to reach that conclusion.

## Decision

What was changed, precisely. For RED-zone deviations: exactly which
invariant or upstream-owned code was touched and how.

## Consequences

- Upstream merge/conflict risk introduced, and how to resolve it at the next
  sync.
- Invariants affected and how they are still preserved (or knowingly
  weakened, and why that was approved).
- Verification performed (tests, `just check`, regeneration commands).

## Links

- Approval (issue/PR/conversation), related ADRs, relevant policy sections.
