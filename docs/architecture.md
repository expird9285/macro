# Architecture Map

A reconnaissance-level map of this repository for developers and coding agents.
It explains where things live, what each major subsystem does, how they
interact, and where to start looking for each kind of change. Companion docs:
`docs/CLOUD_STORAGE.md` (backend/deploy basics), `docs/RUNNING_LOCALLY.md`
(local stack), `docs/STYLE_GUIDE.md` (review conventions), and the root
`CLAUDE.md` / `AGENTS.md` (agent instructions — **read these first**).

## 1. What this repository is

Macro: a cloud-storage / collaboration workspace (documents, email, channels &
chat, calendar, contacts, search, AI agents) built as:

- A **Rust microservice backend** — a single Cargo workspace with ~130 library
  crates (`crates/`) and ~40 deployable services (`services/`, plus a few
  non-Rust services), backed by PostgreSQL, Redis, S3, SQS, Kafka, DynamoDB,
  and OpenSearch.
- A **SolidJS frontend** (`apps/web`) that also ships as a native app via
  Tauri, talking to the backend over HTTP/GraphQL/WebSocket.
- **Edge services** on Cloudflare Workers (collaborative sync, websocket
  fan-out, AI editing) written in Rust-to-WASM and TypeScript.
- **Infrastructure as code** with Pulumi (`infra/`), Nix for the dev shell
  (`flake.nix`), and `just` for task running.

## 2. Top-level layout

| Path | Contents |
| --- | --- |
| `crates/` | Reusable Rust library crates: domain logic, DB clients, service clients, models, shared infra utilities |
| `services/` | Deployable processes: axum HTTP services, SQS/lambda handlers, Kafka consumers, plus TS/Workers and one Python service |
| `apps/web/` | SolidJS + Vite frontend (also the Tauri native app). Has its own `AGENTS.md`/`CLAUDE.md` and justfile |
| `apps/docs/` | Documentation site (MDX) |
| `packages/` | Shared TypeScript packages: `sdk` (OpenAPI-generated + hand-written TS SDK), `collaboration`, `lexical-core`, `loro-mirror`, `observability` |
| `tooling/` | Dev tooling: `just/` (imported justfile modules), `xtask` (Rust dev orchestrator), `seed_cli`, `native_app_server`, `notification_sandbox` |
| `infra/` | Pulumi stacks (one directory per service under `infra/stacks/`) |
| `docker/` | docker-compose files for local databases and the local stack |
| `docs/` | Repo-level documentation (this file, style guide, local-dev guide) |
| `nix/`, `flake.nix` | Pinned dev shell; the only host dependency in cloud dev environments |
| `.claude/skills/` | Project skills: repo-specific workflows (see §10) |

## 3. Cargo workspace structure

The root `Cargo.toml` defines the workspace (resolver 2) with members from
`crates/`, `services/`, and `tooling/`. Notes:

- **Not every directory under `crates/`/`services/` is a workspace member.**
  The member list in the root `Cargo.toml` is explicit; some crates are pulled
  in transitively as path dependencies, and several `services/` directories
  are not Rust at all (see §4.4).
- All third-party versions are pinned in `[workspace.dependencies]`; crates
  reference them with `workspace = true`. Add new dependencies there.
- `crates/workspace-hack` is a hakari-managed crate (`just hakari`); don't
  edit it by hand.
- `crates/client/*` and `crates/agent_fold` compile to WASM for the frontend
  (`apps/web` builds them via `just ensure-cache-wasm` / `ensure-agent-fold-wasm`).

## 4. Major subsystems

### 4.1 Core storage & retrieval (Rust services)

- `services/document_storage_service` (DSS) — the main API service: document
  CRUD, projects, teams, permissions endpoints, and the **GraphQL API**
  (async-graphql; the `graphql_*` crates compose its schema, with
  `crates/graphql_soup` / `crates/soup` powering filtered multi-entity
  queries — "soup" = an amalgamated query service returning many entity types
  by filter).
- `services/static_file_service` — static/raw file serving from S3.
- `services/search_processing_service`, `services/search_upload_handler`,
  `crates/search_service`, `crates/opensearch_client` — OpenSearch indexing
  and query.
- `services/convert_service`, `services/document_text_extractor`,
  `services/docx_unzip_handler`, `services/document_cognition_service` — the
  document processing pipeline: Upload → text extraction (pdfium for PDFs,
  Lambda unzip for DOCX) → search indexing → storage/retrieval.

### 4.2 Communication & collaboration

- `services/email_service` + `crates/email`, `crates/email_db_client`,
  `crates/gmail_client` and the `email_*_handler` lambdas — email sync,
  scheduling, suppression.
- Channels/chat: `crates/channels`, `crates/chat`, `crates/comms_db_client`;
  notifications via `services/notification_service` +
  `crates/notification_db_client`.
- `services/connection_gateway` (Rust) — WebSocket gateway for realtime
  updates (connection tracking in DynamoDB).
- `services/sync-service` (Rust → Cloudflare Workers + Durable Objects) —
  collaborative document sync using Loro CRDTs; one Durable Object "room" per
  document. Bebop schemas in `services/sync-service/bebop-schema`.
- `services/websocket-service`, `services/lexical-service`,
  `services/ai-editing-worker`, `services/analytics-proxy`,
  `services/cla-worker` — TypeScript / Cloudflare Worker services.
- `services/transcription` — Python (LiveKit) call transcription.

### 4.3 AI subsystem

- `crates/ai_toolset` — the AI tool **framework** (schema derivation, toolset
  composition). **Never modify it**; build tools on top of it (see
  `crates/ai_toolset/TOOL_DESIGN.md` and the `create-ai-tool` skill).
- Tools live in domain crates at `crates/<crate>/src/inbound/toolset/`
  (e.g. `crates/documents`, `crates/email`, `crates/soup`, `crates/call`) and
  are aggregated in `crates/ai_tools::all_tools()`.
- Agents: `crates/agent`, `crates/agent_session`, `crates/agent_harness` +
  `services/agent_harness_service` (runs agents in Daytona sandboxes),
  `crates/agent_trigger` + `services/agent_trigger_service`,
  `crates/agent_fold` (WASM-reachable), `crates/anthropic`, `crates/prompt`,
  `crates/skills`/`crates/system_skills`, `crates/memory`.
- MCP: `services/mcp_service`, `services/mcp_auth_proxy`,
  `crates/mcp_client`, `crates/pipedream_mcp`.

### 4.4 Auth & access control

- `services/authentication_service` + `crates/macro_auth`,
  `crates/fusionauth` (FusionAuth-backed; local stack has passwordless login
  that returns the code in the response).
- `crates/entity_access` — hexagonal entity-level access checks; produces
  typed `EntityAccessReceipt<T>` capabilities (see §7 — this is a
  load-bearing convention).
- `crates/macro_authorization`, `crates/roles_and_permissions`,
  `crates/entity_access_management`.

### 4.5 Frontend (`apps/web`)

SolidJS + Vite + Tailwind (semantic tokens) + Kobalte + Lexical (rich text)
+ TanStack Solid Query. Entry: `apps/web/src/index.tsx`; routing under
`src/routes/`; UI organized as `src/features/<feature>` (entity types render
as `block-*` features: `block-md`, `block-email`, `block-channel`, …).
Network calls live in service-clients; shared queries in `src/lib/queries`.
`apps/web/tauri` holds the native shell. GraphQL codegen via
`apps/web/codegen.ts` against `graphql-client-schema.graphql`.
Good reference features per its AGENTS.md: `src/features/entity`,
`src/features/channel`, `src/features/block-md`, `src/features/next-soup`.
Bad examples to not copy: `block-channel`(sic — see its AGENTS.md), `block-pdf`.

### 4.6 TypeScript SDK (`packages/sdk`)

Generated HeyAPI client from the services' OpenAPI specs (`generated/`) plus a
hand-written ergonomic layer (`src/`). Every new backend endpoint must be
wrapped or explicitly skipped (`src/coverage/skipped.ts`); `just coverage`
enforces this — use the `add-sdk-endpoint` skill.

## 5. Databases & migrations

| Store | Client crate | Migrations |
| --- | --- | --- |
| **MacroDB** (main Postgres: documents, users, projects, comms, email, notifications) | `crates/macro_db_client` | `crates/macro_db_client/migrations/` (~290 files; `0001_baseline.sql` + timestamped) |
| ContactsDB | `crates/contacts` | within the crate |
| Comms data (in MacroDB) | `crates/comms_db_client` | uses MacroDB schema; own fixtures |
| Email data | `crates/email_db_client` | own fixtures |
| Notifications | `crates/notification_db_client` | — |
| Redis | `crates/macro_redis`, `crates/macro_cache_client` | n/a |
| OpenSearch | `crates/opensearch_client` | n/a |
| DynamoDB (connections) | `crates/dynamodb_client` | n/a |
| S3 | `crates/s3_client` | n/a |

Key SQLx workflow (see root `CLAUDE.md` for the full rules):

- New migration: `sqlx migrate add <name>` from the DB crate — never
  hand-create or guess timestamp prefixes.
- After changing SQL in Rust: `just prepare_db` **from the repo root, inside
  `nix develop`** — never edit `.sqlx/query-*.json` by hand.
- Tests must run against live local Postgres — never set `SQLX_OFFLINE=true`
  for `cargo test` (offline is fine for check/build/clippy).
- Some columns are camelCased — alias in SQL (`"userId" as "user_id"`); use
  the `/dump-schema` skill to see the actual schema.

## 6. Service-to-service communication

- **HTTP** — internal calls via typed client crates:
  `*_service_client` / `*_api_client` (e.g. `document_storage_service_client`,
  `email_service_client`, `authentication_service_client`). URLs from
  `crates/macro_service_urls`; internal auth via signed headers/JWT
  (`crates/macro_sync_service_jwt`, `crates/decode_jwt`).
- **Kafka events** — `crates/macro_event_broker` (hexagonal producer/consumer
  service) with topics defined in `crates/macro_event_topics`
  (channels, documents, email, projects topics) and shared transports in
  `crates/kafka_util`.
- **SQS queues** — `crates/macro_queues` (typed queue-name newtypes with
  per-environment defaults and `OVERRIDE_*` env overrides), `crates/sqs_client`,
  `crates/sqs_worker`; consumed by the many `*_handler` lambda services.
- **Realtime** — `services/connection_gateway` (WebSocket; DynamoDB
  connection tracking) and `crates/soup_realtime`/`crates/broadcast` for
  pushing entity updates.
- **Env/config** — everything through `crates/macro_env_var` macros (never
  `std::env::var`); secrets managed in Doppler.

## 7. The hexagonal architecture convention (load-bearing)

Newer backend crates follow strict ports-and-adapters, enforced by the
`cloud-storage-hexagonal-architecture` skill
(`.claude/skills/cloud-storage-hexagonal-architecture/SKILL.md` — read it
before touching `crates/**` or `services/**`):

```
src/domain/    models, errors, port traits, domain services  ← ALL business/authz policy
src/inbound/   axum handlers, AI tools, Kafka/lambda listeners — thin adapters only
src/outbound/  SQLx, AWS, Redis, HTTP client implementations of domain ports
```

- Dependencies point inward; `domain/` must not know axum/SQLx/AWS.
- Authorization crosses the boundary as a typed `EntityAccessReceipt<T>`
  minted by access extractors in inbound; handlers must not branch on
  `AccessLevel`/roles — domain services own policy.
- Examples of the pattern: `crates/documents`, `crates/email`, `crates/soup`,
  `crates/entity_access`, `crates/macro_event_broker`.
- Older services (e.g. `services/document_storage_service` with its
  `api/`/`model/`/`service/` layout) predate the convention; don't spread the
  old style, and when touching a use case prefer moving policy into a domain
  service.

## 8. Build, dev & test workflow

All via `just` (root `justfile` imports modules from `tooling/just/`:
`rust.just`, `check.just`, `local_stack.just`, `xtask.just`; DB crates get
`sqlx.just`/`database.just`).

```bash
# Build / check / lint
just build            # build all services
just check            # type check
just clippy           # lints (run before commit, with cargo fmt)
just build_lambdas    # lambda artifacts

# Local infra + tests
just create_networks
just run_dbs -d
just setup_test_envs
just initialize_dbs   # == setup_macrodb
cargo test -p <crate>     # from repo root; SQLX_OFFLINE unset
# there is no `just test`

# DB reset
just crates/macro_db_client/drop_db -y -f && just setup_macrodb

# Running the full product locally: see docs/RUNNING_LOCALLY.md and the
# `run-app` skill (Cursor Cloud: .cursor/infra.sh / stack.sh / rebuild.sh)
```

Frontend (`apps/web`, bun as package manager): `bun run dev`, `bun run test`
(vitest), `bun run check` (tsc), `bun run lint` (biome), `bun run knip`
(dead code). Playwright e2e in `apps/web/tests`.

Repo alias: use `\cd` instead of `cd` when navigating in the repo.

### Test organization

- **Unit/integration per crate** — tests go in a separate `foo/test.rs`
  submodule (`#[cfg(test)] mod test;` in `foo.rs`), not inline blocks.
  DB-backed tests use fixtures under each DB crate (`fixtures/`) and need the
  local Docker databases running.
- **Local E2E (Rust)** — `crates/integration_tests/local_e2e`, run with
  `just local-e2e-rust` (tests are `#[ignore]`d so plain `cargo test` doesn't
  need Docker); orchestrated by `tooling/xtask` with seed data shared with
  `tooling/seed_cli` and Playwright (`just seed-scenario`).
- **Frontend** — vitest colocated with source; Playwright in `apps/web/tests`.
- **QC gate** — the `qc` skill runs 5 parallel review agents over a diff.

## 9. Where to start looking, by type of change

| Change | Start here |
| --- | --- |
| New/changed HTTP endpoint | Domain crate's `src/inbound/` + domain service in `src/domain/`; router wiring in the owning service under `services/`; then wrap it in `packages/sdk` (add-sdk-endpoint skill) |
| Business rule / permission logic | The domain service in `crates/<domain>/src/domain/` — never in handlers (§7) |
| DB schema change | `sqlx migrate add` in `crates/macro_db_client` → migrate → `just prepare_db` → update tests/fixtures |
| New DB query | `src/outbound/` of the owning crate, SQLx macros (`query!`/`query_as!`), then `just prepare_db` |
| New AI tool | `create-ai-tool` skill; tool in `crates/<domain>/src/inbound/toolset/`, wired into `crates/ai_tools` |
| Search behavior | `crates/search_service`, `crates/opensearch_query_builder`, `services/search_processing_service` |
| Realtime/eventing | `crates/macro_event_topics` (topics), `crates/macro_event_broker`, `crates/soup_realtime`, `services/connection_gateway` |
| Email | `crates/email` (domain) / `services/email_service` / `crates/email_db_client` |
| Frontend UI | `apps/web/src/features/<feature>` (blocks per entity type); follow `apps/web/AGENTS.md` patterns |
| GraphQL | `crates/graphql_*` schema crates, served from `services/document_storage_service`; frontend codegen `apps/web/codegen.ts` |
| Collaborative editing/sync | `services/sync-service` (Durable Objects), `packages/collaboration`, `packages/loro-mirror`, `crates/client/*` |
| Deployment/infra | `infra/stacks/<service>` (Pulumi) |
| Local dev environment | `docs/RUNNING_LOCALLY.md`, `tooling/just/`, `tooling/xtask`, `docker/` |

## 10. Extension points & customization safety

**Designed extension points (safe):**

- New AI tools via `ai_toolset` (framework untouched).
- New domain crates following the hexagonal template; new inbound adapters
  calling existing domain services.
- New Kafka topics in `macro_event_topics`; new queues via the
  `macro_queues::queue!` macro; new env vars via `macro_env_var` macros
  (added to Doppler).
- New SDK wrappers in `packages/sdk/src`; new frontend features under
  `apps/web/src/features`.
- Project skills in `.claude/skills/` document the sanctioned workflows:
  `create-ai-tool`, `add-sdk-endpoint`, `dump-schema`, `run-app`,
  `debug-service`, `upgrade-model`, `qc`, `cloud-storage-hexagonal-architecture`.

**Tightly coupled / risky (change with care, or don't):**

- `crates/ai_toolset` — framework; explicitly off-limits for modification.
- `crates/macro_db_client/migrations` — shared by many services; schema
  changes ripple through `.sqlx` caches, fixtures, and camelCase quirks.
- `crates/entity_access` / `EntityAccessReceipt` semantics — the authorization
  backbone; every service depends on its contract.
- `crates/workspace-hack` (generated), `.sqlx/` metadata (generated),
  `packages/sdk/generated` (generated) — regenerate, never hand-edit.
- `services/sync-service` Durable Object state & bebop schemas — persisted
  CRDT state; schema/versioning mistakes corrupt documents. Similarly the
  frontend Lexical node versioning (`apps/web/.../LexicalMarkdown/version.ts`).
- `services/document_storage_service` — large, pre-hexagonal, central; the
  GraphQL schema and permissions endpoints have many consumers.
- Root `Cargo.toml` workspace deps and `flake.nix` pins — affect everyone.

## 11. Agent instruction files

- `/CLAUDE.md` (root; `/AGENTS.md` points to it) — build/test commands, SQLx
  rules, error handling (`rootcause` preferred over `anyhow`), tracing
  conventions, test file layout, doc requirements, Cursor Cloud scripts.
- `apps/web/CLAUDE.md` → `apps/web/AGENTS.md` — SolidJS/Kobalte/Lexical
  patterns, styling rules, iOS worker gotcha.
- `docs/STYLE_GUIDE.md` — numbered review conventions (CS-xx) for Rust and TS.
- `.claude/skills/*/SKILL.md` — per-workflow instructions (see §10).
