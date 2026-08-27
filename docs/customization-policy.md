Customization Policy

You are defining the customization and change policy for a long-lived fork of the Macro repository.

The purpose of this document is NOT to describe how Macro works in general.

Its purpose is to establish clear boundaries for future coding agents and developers:

- what they should modify,
- what they may modify only when necessary,
- what they should avoid modifying,
- what they must never edit directly,
- how they should investigate the repository before making a classification,
- how they should control change scope,
- and when they must stop and escalate instead of making an architectural judgment call.

The repository is a long-lived fork of the upstream Macro project.

The primary engineering goals are:

1. Preserve upstream compatibility where practical.
2. Minimize unnecessary divergence from upstream.
3. Prefer existing extension points over invasive architectural changes.
4. Keep custom functionality isolated from upstream-owned code whenever practical.
5. Make future upstream synchronization and conflict resolution easier.
6. Prevent coding agents from expanding the scope of a task without explicit justification.
7. Never weaken repository-enforced architectural, authorization, database, API, or generated-artifact invariants.

Companion docs:

- "docs/architecture.md" — where things live and architectural boundaries.
- "docs/development.md" — verified development and test procedures.
- "docs/STYLE_GUIDE.md" — review and code-quality rules.
- root "CLAUDE.md" / "AGENTS.md" — repository-specific agent instructions.

---

0. Investigation Before Classification

Before modifying code, classify a task or repository area only after sufficient targeted investigation.

A coding agent MUST inspect the repository structure needed to establish the classification. Do not classify an area merely from filename conventions, assumptions, or generic knowledge of Macro.

At minimum, inspect as applicable:

- "CLAUDE.md"
- "AGENTS.md"
- "docs/architecture.md"
- "docs/development.md"
- repository documentation relevant to the task
- workspace configuration
- Cargo manifests
- major crates
- frontend/application structure
- database-related crates and migrations
- generated or derived files
- build and code-generation scripts
- existing extension/customization mechanisms

Do NOT read the entire repository indiscriminately.

Use targeted search and inspect only the files necessary to determine:

- ownership,
- architectural boundaries,
- extension points,
- dependency relationships,
- source-of-truth files,
- generation workflows,
- verification requirements,
- and upstream-maintenance risk.

Do not modify application source code merely for the purpose of investigating classification.

If an area cannot be confidently classified from the repository, mark it:

«Needs further investigation»

and identify exactly what must be investigated before proceeding.

---

1. Guiding Principles

1. Additive over invasive. Prefer new crates, modules, adapters, features, migrations, endpoints, frontend features, or configuration over editing shared upstream code in place. Small upstream diffs are easier to maintain and merge.

2. Use the designed extension points first. The repository already provides sanctioned mechanisms for most customization. Only touch shared code when no suitable extension point covers the requirement.

3. Make the smallest complete change. A coding agent must make the smallest change that completely satisfies the approved task.

4. Preserve enforced invariants. Hexagonal boundaries, the "EntityAccessReceipt" authorization contract, SQLx/migration discipline, generated-artifact freshness, SDK endpoint coverage, and other repository-enforced rules must remain intact.

5. Document deliberate deviations. Any significant upstream divergence, RED-zone modification, architectural decision, or exceptional scope expansion must be recorded.

6. Optimize for future synchronization. When two implementations satisfy the task equally well, prefer the implementation with lower upstream merge and conflict-resolution risk.

7. Do not confuse technical preference with task justification. A technically "better" implementation is not sufficient reason to broaden scope, replace an existing design, refactor nearby code, or alter upstream behavior.

---

2. Zone Classification

The following classification is the default repository-wide model.

GREEN — PREFER MODIFYING

GREEN areas are designed extension points or fork-friendly areas. Customization is preferred here when the task fits the intended mechanism.

Area| How
New backend feature| New domain crate or new module in the owning domain crate following "domain/" + "inbound/" + "outbound/"; wire it into the owning service. Follow the "cloud-storage-hexagonal-architecture" skill
New HTTP endpoint| New inbound adapter calling a domain service; wrap it in "packages/sdk" using the "add-sdk-endpoint" skill, or explicitly record it as skipped
New AI tool| "crates/<domain>/src/inbound/toolset/" on top of "ai_toolset", aggregated in "crates/ai_tools" using the "create-ai-tool" skill
New DB tables/columns| Additive migration via "sqlx migrate add" in the DB crate, then "just prepare_db"
New Kafka topic / SQS queue / environment variable| "crates/macro_event_topics" / "macro_queues::queue!" / "macro_env_var" macros; environment variables are also registered in Doppler
New frontend feature| "apps/web/src/features/<feature>" following "apps/web/AGENTS.md"; entity UIs use the established "block-*" feature patterns where appropriate
New SDK surface| Hand-written wrappers in "packages/sdk/src"
New seed scenarios, skills, documentation| "seed/scenarios/", ".claude/skills/", "docs/"
Fork-specific UI/configuration| Only where repository structure explicitly provides an appropriate configuration or customization boundary

GREEN does not mean "change anything freely."

Even in GREEN, the agent must:

- understand the relevant local convention,
- stay within the approved task scope,
- avoid unrelated cleanup,
- verify the change appropriately,
- and avoid creating unnecessary upstream divergence.

---

YELLOW — MODIFY WHEN NECESSARY

YELLOW areas may require changes for substantial fork-specific functionality, but modifications have meaningful architectural, dependency, or upstream-maintenance cost.

Examples include:

- existing domain crates and services,
- shared Rust crates,
- service interfaces,
- database-backed functionality,
- authentication-related behavior,
- shared frontend infrastructure,
- GraphQL schema,
- workspace dependency configuration,
- existing migrations' surrounding queries/fixtures,
- cross-service communication,
- existing application logic without a dedicated extension point.

Rules:

- Touch only what the task requires.
- Keep the diff narrowly focused.
- Prefer adding behavior over rewriting existing behavior.
- Move business or authorization policy inward into the owning domain/service rather than spreading it across adapters.
- Do not modify shared infrastructure merely because doing so is convenient.
- Investigate all relevant consumers before changing shared behavior.
- Verify all affected boundaries.

Examples:

Existing domain crates and services

Extend the owning domain service rather than putting business logic into adapters.

If touching a pre-hexagonal use case, move policy inward rather than expanding the old architectural pattern.

Required investigation:

- owning service,
- domain service,
- inbound adapters,
- outbound adapters,
- known consumers,
- tests,
- relevant extension points.

Verification:

- targeted crate tests,
- lint/formatting,
- integration tests where service boundaries are affected,
- broader checks when shared behavior is modified.

Workspace dependencies

Adding a genuinely task-related dependency may be acceptable.

Before adding one, determine:

- whether equivalent functionality already exists,
- whether an existing dependency can be reused,
- the correct crate/service ownership,
- licensing implications where relevant,
- maintenance implications,
- and upstream synchronization impact.

Do not add dependencies merely to simplify a small implementation.

Upgrading a shared workspace dependency is higher risk than adding an isolated dependency and should be isolated and justified.

GraphQL

GraphQL schema changes should be additive whenever possible.

Before changing a shared schema, identify:

- all known consumers,
- generated frontend types/codegen,
- compatibility requirements,
- affected tests,
- upstream impact.

Breaking changes require explicit architectural review.

---

RED — AVOID MODIFYING / STOP BEFORE MODIFYING

RED areas should remain aligned with upstream unless the fork has an explicit and documented reason to diverge.

The following are explicitly off-limits without higher-level approval:

- "crates/ai_toolset" — the AI tool framework
- "crates/entity_access" semantics / the "EntityAccessReceipt<T>" contract
- existing migration files in any "migrations/" directory
- generated artifacts
- "crates/workspace-hack"
- "packages/sdk/generated"
- files marked "DO NOT EDIT"
- generated CI workflows that must be regenerated with repository tooling
- "services/sync-service" persisted Durable Object state and associated schema contracts
- Lexical node formats without the required version handling
- "flake.nix" / "nix/" toolchain pins
- ".cursor/*.sh" entry points
- authorization branching in inbound adapters, including access-level, role, or ownership checks in handlers

Other core infrastructure, foundational abstractions, unrelated services, build infrastructure, and shared infrastructure should also be treated as RED unless the repository investigation demonstrates a legitimate requirement.

Do not modify RED areas merely because:

- the implementation would be easier,
- the existing design appears suboptimal,
- a refactor would make the code "cleaner",
- a new abstraction would look more elegant,
- or the agent can find a technically valid implementation.

If a RED-area change appears necessary, STOP and escalate.

---

3. GENERATED / VENDOR / DERIVED CONTENT

Generated or derived content must not be edited manually.

Examples include:

- ".sqlx/query-*.json"
- "crates/workspace-hack"
- "packages/sdk/generated"
- generated CI workflows
- files explicitly marked "DO NOT EDIT"
- other generated artifacts identified by repository tooling

However, an agent must NOT assume that a file is generated merely because of its name.

For each generated or derived area, establish:

1. What generates it?
2. Where is the source of truth?
3. What command or workflow regenerates it?
4. What verification confirms freshness?
5. What source files should be modified instead?

Examples:

SQLx metadata

Source of truth:

- database schema,
- migrations,
- SQL query sources.

Regeneration:

- follow the repository's documented SQLx preparation workflow,
- use "just prepare_db".

Never manually edit ".sqlx/query-*.json".

Workspace-hack

Generated/derived from workspace dependency state.

Modify the relevant workspace dependency configuration instead, then regenerate using the documented repository workflow.

SDK generated output

Modify the source schema/codegen input or the hand-written SDK surface as appropriate, then regenerate.

Never patch generated output directly.

---

4. Extension-Point-First Policy

Before modifying core or shared infrastructure, determine whether an existing extension point can satisfy the requirement.

The preferred order is:

1. Existing configuration
2. Existing extension/plugin mechanism
3. Existing module or service boundary
4. Local feature-specific implementation
5. Shared implementation
6. Core infrastructure modification

An agent must choose the earliest viable option.

If a more invasive option is selected, the implementation plan must document:

- why the earlier extension points do not satisfy the requirement,
- what boundary must be crossed,
- why the additional complexity is justified,
- and what upstream-maintenance cost it creates.

---

5. Change-Scope Policy

A coding agent must begin each task with an explicit expected component/file scope.

The agent must NOT:

- refactor unrelated code,
- rename unrelated symbols,
- reorganize files for aesthetic reasons,
- "clean up" nearby code,
- replace an implementation merely because another approach appears better,
- introduce abstractions without a concrete requirement,
- modify unrelated services,
- change public/shared interfaces unnecessarily,
- update dependencies without a task-related reason,
- or alter upstream behavior unrelated to the requested customization.

If implementation requires files or components outside the expected scope:

1. determine why they are required,
2. verify that the dependency is real,
3. identify the affected ownership/boundary,
4. update the implementation plan,
5. evaluate whether the expansion materially changes risk,
6. escalate when it introduces an architectural decision.

Do not silently expand scope.

---

6. Dependency-Impact Policy

Before making a cross-boundary change, identify the dependency chain.

Determine:

- which component owns the behavior,
- which components depend on it,
- which interfaces are affected,
- whether the change is backward-compatible,
- which tests cover the affected behavior,
- whether database, frontend, SDK, or service consumers are involved,
- and whether upstream synchronization will become harder.

Cross-service or cross-crate changes must not be made merely because they are convenient.

If the dependency chain is unclear, stop and investigate before modifying code.

---

7. Database Policy

Database changes are higher-risk customizations than isolated UI or application changes.

Before changing database-related code, identify:

- affected schema,
- applicable migrations,
- affected services/crates,
- affected SQL queries,
- SQLx metadata,
- fixtures or seed data,
- relevant tests,
- compatibility implications,
- rollback considerations where applicable.

Rules:

- Schema changes use additive migrations created with "sqlx migrate add".
- Existing migration files are append-only and must not be edited or renamed.
- Do not invent timestamp prefixes manually.
- Queries use compile-checked macros such as "query!", "query_as!", or "query_scalar!" where required by repository conventions.
- After SQL changes, run the repository's database preparation workflow.
- Tests must run against the required live local database environment.
- Fixtures must remain consistent with schema changes.

After any relevant SQL change:

- run "just prepare_db",
- run the affected tests,
- verify generated metadata freshness,
- and perform broader verification when multiple services depend on the schema.

Never manually edit generated SQLx metadata.

---

8. API and Interface Policy

Treat changes to shared interfaces as higher-risk changes.

This includes:

- service APIs,
- internal RPC/interfaces,
- frontend/backend contracts,
- database-facing interfaces,
- GraphQL schemas,
- shared Rust crate interfaces,
- public SDK APIs.

Before changing an interface, determine:

- all known consumers,
- compatibility requirements,
- migration steps,
- tests,
- generated code implications,
- frontend codegen implications,
- upstream impact.

Prefer additive and backward-compatible changes.

Do not introduce breaking changes merely for implementation convenience.

Every new endpoint should either:

- be wrapped in "packages/sdk/src", or
- be explicitly recorded in "packages/sdk/src/coverage/skipped.ts".

The relevant SDK coverage verification must pass.

---

9. Dependency-Addition Policy

Do not introduce a new third-party dependency unless the task genuinely requires it.

Before adding one, determine:

- whether equivalent functionality already exists,
- whether an existing workspace dependency can be reused,
- where the dependency belongs,
- whether it affects multiple crates or services,
- maintenance implications,
- licensing implications where relevant,
- and upstream merge implications.

Prefer reusing existing dependencies where appropriate.

Do not add a dependency merely to make a localized implementation easier.

---

10. Backend Structure and Architecture Policy

New Rust code follows the repository's established hexagonal structure.

Business logic and authorization policy belong in the domain layer.

Adapters must remain thin.

Do not introduce authorization decisions into inbound adapters.

For new crates:

- follow repository crate conventions,
- use required documentation/lint settings,
- use repository-standard error handling,
- use the workspace dependency table,
- follow repository test layout,
- run the relevant architectural skill checklist.

When modifying an existing pre-hexagonal implementation, prefer moving policy inward rather than spreading the older structure to new code.

---

11. Configuration Policy

All environment variables must use the repository's configuration mechanism.

Rules include:

- use "macro_env_var",
- do not use "std::env::var" for repository configuration,
- register new variables in Doppler,
- use "macro_queues::queue!" for queues,
- use "macro_event_topics" for event topics,
- use "macro_service_urls" for service URLs.

Treat configuration semantics as part of the existing public behavior unless the task explicitly requires divergence.

---

12. Frontend Policy

New frontend functionality belongs under:

"apps/web/src/features/<feature>"

Follow "apps/web/AGENTS.md".

Prefer composition over unnecessary configurability.

Follow repository conventions for:

- semantic color tokens,
- derived state,
- network service clients,
- shared queries,
- feature boundaries,
- entity/block feature organization.

Use established references such as:

- "features/entity"
- "features/channel"
- "features/block-md"
- "features/next-soup"

Do not copy unsuitable patterns such as "block-channel" or "block-pdf" when repository guidance explicitly identifies them as non-reference implementations.

For user-visible frontend changes, verification should include:

- frontend checks,
- relevant tests,
- browser/local-stack verification when behavior cannot be validated adequately through static checks alone.

---

13. Upstream-Divergence Policy

This is a long-lived fork.

Every significant customization must be evaluated for upstream divergence.

Prefer changes that:

- remain isolated,
- preserve upstream interfaces,
- minimize modification of upstream-owned code,
- avoid unnecessary renaming,
- avoid unnecessary file movement,
- preserve existing behavior outside the task,
- and can be cleanly reapplied or reconciled after upstream updates.

When two implementations satisfy the requirement equally well, prefer the one with lower upstream merge risk.

When divergence is required, prefer:

- new code paths,
- new modules,
- new endpoints,
- feature flags,
- additive migrations,
- fork-specific components,

over directly rewriting the upstream path in place.

---

14. Custom Code Ownership

Where practical, fork-specific functionality should be identifiable as fork-specific.

If the repository provides an existing location for custom functionality, use that location.

Document established conventions for:

- custom backend modules,
- custom frontend features,
- custom configuration,
- custom services,
- fork-specific database objects,
- fork-specific extension mechanisms.

Do not invent a new organizational convention when an existing repository convention is suitable.

---

15. Verification Policy

Verification should be proportional to the change.

The smallest relevant verification should be performed first.

At minimum distinguish between:

Change type| Minimum verification
Isolated UI change| targeted frontend checks/tests; browser verification when behavior is user-visible
Frontend application logic| frontend checks and relevant tests
Rust crate change| "cargo fmt", targeted crate tests, relevant lint/clippy checks
Cross-crate change| affected crate tests plus boundary/integration verification
Service/API change| affected service tests, SDK coverage, local-stack verification where relevant
Database change| "just prepare_db", affected tests against live DB, generated-artifact verification
Configuration change| targeted configuration/build/test verification
Generated-content change| regenerate from source of truth and verify freshness

Repository-wide checks should be used when:

- repository rules require them,
- shared/core behavior changes,
- multiple subsystems are affected,
- or the risk warrants broader verification.

Do not require expensive repository-wide verification for every trivial isolated change unless repository policy mandates it.

Repository baseline verification includes, where applicable:

- "cargo fmt"
- "just clippy"
- "cargo test -p <touched crates>"
- "just check" or "just check full"
- "just prepare_db"
- "just hakari" after dependency/workspace changes
- SDK coverage
- frontend "bun run check"
- frontend "bun run test"
- local-stack/browser verification for user-visible behavior

Use "docs/development.md" as the authoritative source for exact commands.

---

16. Documentation Consistency

This policy must remain consistent with:

- "CLAUDE.md"
- "AGENTS.md"
- "docs/architecture.md"
- "docs/development.md"
- "docs/STYLE_GUIDE.md"
- task specifications
- repository development conventions

If these documents conflict, do NOT silently choose one.

Identify:

1. the conflicting rules,
2. the affected decision,
3. the possible interpretations,
4. and the required human/model decision.

Escalate instead of inventing a precedence rule.

---

17. Agent Decision Procedure

Before writing code, classify the task.

1. Investigate the relevant repository structure.
2. Identify the owning component and existing extension points.
3. Does a GREEN extension point cover it?
   - Yes → use it.
4. Does the task require editing existing shared code?
   - Yes → classify it as YELLOW and touch the minimum necessary.
5. Does it touch a RED item?
   - Yes → stop and request explicit approval/higher-level analysis.
6. Does the task cross multiple major subsystems unexpectedly?
   - Yes → stop and reassess the architecture.
7. Is the dependency chain unclear?
   - Yes → stop and investigate before modifying code.
8. Is there a materially different architectural choice?
   - Yes → escalate before implementation.
9. Are generated files involved?
   - Modify the source of truth and regenerate; never patch generated output.
10. Does the change expand beyond the approved file/component scope?

- Verify the dependency, update the plan, and escalate when risk materially increases.

---

18. Agent Stop Conditions

A coding agent must STOP rather than continue autonomously when:

- the required change crosses several major subsystems unexpectedly,
- the dependency chain is unclear,
- a core architectural component must be modified,
- a breaking API/interface change appears necessary,
- a significant database redesign appears necessary,
- a new foundational dependency is required,
- generated files appear to require manual modification,
- the task conflicts with repository instructions,
- the approved scope is no longer sufficient,
- multiple materially different architectural approaches exist,
- an authorization invariant would need to change,
- a RED-zone item appears unavoidable,
- or the correct ownership of the behavior cannot be established confidently.

When stopping, the agent must report:

1. What it discovered
2. Why the current plan is insufficient
3. The affected components
4. The available options
5. The risks of each option
6. What decision is required

The agent must NOT choose an architecture merely because it can implement it.

---

19. Escalation Policy

LOW RISK

The change is:

- isolated,
- well understood,
- within approved scope,
- based on an existing extension point,
- and does not alter shared architecture.

A CHEAP implementation model may proceed after ordinary verification.

MEDIUM RISK

The change:

- touches multiple files/components,
- modifies shared logic,
- affects an interface,
- changes database-backed behavior,
- or has moderate upstream implications.

Require stronger planning and review before implementation.

A stronger planning/model pass should occur before making the change if the architecture is not already unambiguous.

HIGH RISK

The change:

- modifies core infrastructure,
- changes database architecture,
- introduces breaking interfaces,
- crosses multiple services,
- introduces substantial upstream divergence,
- changes authorization contracts,
- introduces a foundational dependency,
- or requires a new architectural pattern.

Escalate to BIG MODEL before implementation.

Do not proceed autonomously until the architectural decision is explicit.

---

20. Fork Hygiene

- Keep customizations in focused commits.
- Use Conventional Commit titles such as "feat(scope): ...".
- Do not reformat unrelated code.
- Do not rename unrelated symbols.
- Do not reorganize unrelated files.
- Do not "clean up" upstream files that are not otherwise part of the task.
- Do not silently change upstream behavior.
- Prefer a new code path over editing the existing upstream path in place when equivalent.
- Keep the custom diff easy to identify during upstream synchronization.
- Record deliberate deviations.

---

21. Enforcement

Repository enforcement may include:

- "just check"
- rustfmt
- biome
- oxlint
- ast-grep rules
- clippy with warnings treated as errors where configured
- nextest
- generated-file freshness checks
- SDK coverage
- repository-specific architecture skills

Relevant skills include:

- "cloud-storage-hexagonal-architecture"
- "create-ai-tool"
- "add-sdk-endpoint"
- "qc"

This policy must be consulted before modifying:

- "crates/**"
- "services/**"
- "apps/web/**"
- "packages/**"

Agents must obey stronger repository-local instructions when applicable.

---

22. Recorded Deviations

Any deliberate RED-zone change or material architectural deviation must be recorded here.

Date| Area| Change| Reason
—| —| none yet| —