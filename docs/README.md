# Documentation Index

Single entry point for repository documentation. Files are grouped by
ownership, not by directory — upstream-owned files stay where upstream put
them so merges stay clean. Do not move or rename existing docs.

## Fork governance (fork-owned — we maintain these)

| Doc | Purpose | Update when |
| --- | --- | --- |
| [customization-policy.md](customization-policy.md) | **Binding** customization rules: GREEN/YELLOW/RED zones, investigation-before-classification, change-scope limits, escalation. Agents follow its Decision Procedure before writing code | A zone judgment changes, or a RED-zone deviation is approved |
| [architecture.md](architecture.md) | Repository map: where subsystems live, how they interact, where to start per type of change | A subsystem is added/removed or a boundary moves |
| [development.md](development.md) | Verified build/test/DB/SQLx/frontend commands and documented failure modes | justfiles, CI workflows, or dev scripts change (re-verify commands against source) |
| [decisions/](decisions/) | Architecture Decision Records: one file per RED-zone deviation or significant architectural/upstream-divergence decision | Immediately, as part of the change that needs one |

## Upstream docs (upstream-owned — edit only to fix drift, keep diffs minimal)

| Doc | Purpose |
| --- | --- |
| [CLOUD_STORAGE.md](CLOUD_STORAGE.md) | Backend layout basics, test prerequisites, deployment pointers |
| [RUNNING_LOCALLY.md](RUNNING_LOCALLY.md) | Local stack: `run_local`, headless `stack`, seeding, instances/ports, troubleshooting |
| [STYLE_GUIDE.md](STYLE_GUIDE.md) | Numbered review conventions (CS-xx/FE-xx), enforced via `rules/ast-grep` and CI |
| [AGENT_SANDBOX_SIZE_PLAN.md](AGENT_SANDBOX_SIZE_PLAN.md) | Upstream plan document |
| [PROPERTY_TARGET_ENTITY_TYPE_PLAN.md](PROPERTY_TARGET_ENTITY_TYPE_PLAN.md) | Upstream plan document |

Related, outside `docs/`: root `CLAUDE.md` / `AGENTS.md` (agent instructions),
`CONTRIBUTING.md`, `apps/web/AGENTS.md` (frontend conventions),
`.claude/skills/*/SKILL.md` (sanctioned workflows), `infra/README.md`.

## Plans

New fork-authored plan documents go in `docs/plans/` (create on first use).
The two existing upstream `*_PLAN.md` files above stay at `docs/` root.

## Precedence on conflict

When documents disagree, higher wins; per the customization policy's
Documentation Consistency rule, report the conflict rather than silently
picking one:

1. Root `CLAUDE.md` / `AGENTS.md`
2. `docs/customization-policy.md`
3. `docs/architecture.md`, `docs/development.md`
4. Upstream docs (`CLOUD_STORAGE.md`, `RUNNING_LOCALLY.md`, `STYLE_GUIDE.md`, plans)
