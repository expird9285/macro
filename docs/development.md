# Development Workflow

Verified reference for coding agents. Every command below exists in the
repository's justfiles, package scripts, CI workflows, or docs
(`docs/RUNNING_LOCALLY.md`, `docs/CLOUD_STORAGE.md`, root `CLAUDE.md`,
`apps/web/AGENTS.md`) as of this writing. Root justfile imports live in
`tooling/just/` (`rust.just`, `check.just`, `local_stack.just`, `xtask.just`,
`sqlx.just`, `database.just`).

## 1. Required environment

- **Nix** is the only required host install. The pinned dev shell supplies
  `just`, Cargo/Rust toolchain, Bun, `wasm-pack`, sqlx-cli, zig,
  cargo-zigbuild, and (on Linux) the Docker CLI + daemon:

  ```bash
  nix develop
  # if it fails:
  nix develop --extra-experimental-features nix-command --extra-experimental-features flakes
  ```

- **macOS**: additionally needs a Docker runtime (Docker Desktop, OrbStack,
  or Colima) — the shell supplies only the CLI.
- **Docker** is needed for local databases and the local stack.
- Doppler is optional; the local stack runs with `--no-doppler` stubs.
- Tauri work uses separate shells: `nix develop .#tauri-linux` /
  `nix develop .#tauri-android`.
- Repo convention: use `\cd` instead of `cd` when navigating (root CLAUDE.md).
- Local dev `DATABASE_URL` (from `tooling/just/database.just`):
  `postgres://user:password@localhost:5432/macrodb`.

## 2. Build commands

Run from the repository root:

```bash
just build              # SQLX_OFFLINE=true cargo build (all services)
just check              # scoped quality gate — see §5 (NOT a cargo check)
just rust-check         # cargo check --workspace --all-features (offline, -Dwarnings)
just build_lambdas      # build every lambda artifact (needed before Pulumi deploys)
just hakari             # regenerate workspace-hack + dep maps after Cargo.toml changes (CI fails if stale)
```

`SQLX_OFFLINE=true` is fine (and used by the recipes) for
check/build/clippy — never for tests (§6).

## 3. Local services

Full product stack (Docker infra + Rust services + proxy + frontend), from
`docs/RUNNING_LOCALLY.md`:

```bash
just doctor-local                # preflight: Docker, toolchain, port availability
just run_local --no-doppler      # attached stack without Doppler (stubbed integrations)
just run_local                   # with Doppler access (lcl_personal config)
```

While attached: press `r` to rebuild changed Rust services, `q` to stop and
clean up (always `q`, not closing the terminal). App URL is printed on
startup; passwordless login codes land in Mailpit at http://localhost:8025.

Headless variant (same flags):

```bash
just stack up                    # bring up, print URLs, detach
just stack status --json
just stack update                # rebuild+reload changed services
just stack update --frontend
just stack down                  # remove containers, volumes, state
```

Other verified controls: `just status_local`, `just stop_local`,
`just destroy_local`, `just reset_local`; multiple stacks via
`--instance <name>` (optionally `--port-base <n>` — then use the **same two
flags on every command**, including `seed-scenario`). `just run_dev` runs
local binaries against shared dev cloud resources (needs Doppler).

Seed data (recommended for a usable stack):

```bash
just seed-scenario apply  --file seed/scenarios/team-perms.json
just seed-scenario status --file seed/scenarios/team-perms.json
just seed-scenario reset  --file seed/scenarios/team-perms.json
```

Cursor Cloud sessions use `bash .cursor/infra.sh` (Docker+Postgres+Redis),
`bash .cursor/stack.sh` (full product), `bash .cursor/rebuild.sh` (remount
rebuilt backend binaries) — see root `CLAUDE.md` and the `run-app` skill.

## 4. Databases

One-time / after resets, from the repo root:

```bash
just create_networks       # docker networks + volumes
just run_dbs -d            # starts Postgres and Redis only (compose --wait)
just setup_test_envs       # writes DATABASE_URL .env into every DB-backed crate
just initialize_dbs        # == setup_macrodb: create MacroDB + run migrations
```

Reset MacroDB:

```bash
just crates/macro_db_client/drop_db -y -f
just setup_macrodb
# if migrations still fail: just crates/macro_db_client/force_drop_db, then setup_macrodb
```

Migrations live in `crates/macro_db_client/migrations/`. Schema reference:
the `/dump-schema` skill. Some table/column names are camelCased — alias in
SQL: `SELECT "userId" as "user_id" ...`.

## 5. Formatting & linting

```bash
cargo fmt                  # or: just format
just clippy                # -Dwarnings, -Dclippy::disallowed_methods, offline
just check                 # fast gate scoped to files changed vs origin/main:
                           #   rustfmt + biome + oxlint + ast-grep (rule ids → docs/STYLE_GUIDE.md)
just check full            # also tsc (apps/web) + clippy (minutes)
```

Pre-commit expectation (root CLAUDE.md): `cargo fmt` + `just clippy`.
In a shallow clone, `just check` has no `origin/main` merge-base and silently
narrows to uncommitted work — set `CHECK_BASE=<ref>` to widen.

Frontend: `bun run fix` (biome write), `bun run lint:ci` from the root, or in
`apps/web`: `bun run lint`, `bun run format`, `bun run knip`.

## 6. Tests

```bash
# prerequisites once per environment (see §4): create_networks, run_dbs -d,
# setup_test_envs, initialize_dbs
cargo test -p <crate>      # from the repo root; SQLX_OFFLINE must be UNSET
```

- **There is no `just test`.** CI runs `cargo nextest run` with a
  change-derived filterset (`.github/workflows/code_check_cloud_storage.yml`),
  after `just setup_test_envs && just initialize_dbs`.
- Never run tests with `SQLX_OFFLINE=true` — tests validate queries against
  live local Postgres; offline mode causes confusing "type annotations
  needed" errors or hides schema drift.
- Pure-logic crates need no Docker at all.
- Local E2E (Rust): `just local-e2e-rust` — self-orchestrating isolated
  stack; the tests are `#[ignore]`d so plain `cargo test` skips them. Also
  `just local-e2e`, `just local-e2e-all`, `just local-e2e-ui` (Playwright).
- Frontend unit tests: `bun run test` (vitest) in `apps/web`;
  `just test-watch` for watch mode.
- SDK endpoint coverage: `just coverage` in `packages/sdk` (every generated
  endpoint must be wrapped or listed in `src/coverage/skipped.ts`; use the
  `add-sdk-endpoint` skill when it fails).

## 7. SQLx workflow

1. **New migration**: `sqlx migrate add <descriptive_name>` from the DB crate
   (e.g. `crates/macro_db_client`), then edit the generated file. Never
   hand-create migration files or invent timestamp prefixes.
2. **Apply**: `just setup_macrodb` (or `just crates/macro_db_client/migrate_db`).
3. **After changing any SQL in Rust**: run `just prepare_db` — **inside
   `nix develop`, from the repository root only**
   (`nix develop --command just prepare_db`). It runs
   `cargo sqlx prepare --workspace` (all features, excluding `sync_service`)
   and updates `.sqlx/`. Never edit `.sqlx/query-*.json` by hand.
4. **Test** with live Postgres (`SQLX_OFFLINE` unset). If tests fail with
   sqlx "no cached data" errors, run `just prepare_db` (add `--tests` when
   the failing query is in test code) — do not enable offline mode.
5. Prefer compile-checked macros (`query!`, `query_as!`, `query_scalar!`)
   over dynamic `sqlx::query`. Update tests and fixtures whenever a DB crate
   changes, and re-run prepare.

## 8. Frontend workflow (`apps/web`)

Package manager and runner is **bun** (workspace root has `bun install`).

```bash
# frontend-only against hosted *-dev services (no Docker needed):
bun install
cd apps/web
bun run dev        # builds cache/agent-fold wasm on first run, then vite

bun run test       # vitest
bun run check      # graphql cache schema check + tsc + biome
bun run lint       # biome lint
bun run format     # biome format --write
bun run knip       # dead-code check
```

Full-stack changes: use `just run_local` (§3) — it serves the frontend too,
with hot reload. GraphQL client codegen config: `apps/web/codegen.ts`.
Follow `apps/web/AGENTS.md` for SolidJS/component/styling conventions
(semantic color tokens, no `createEffect` for derivation, Lexical node
version counter in `src/lib/core/component/LexicalMarkdown/version.ts`).

## 9. Verification procedure (before considering a change done)

Backend (Rust):

1. `cargo fmt` and `just clippy` pass.
2. If SQL changed: migrations applied + `just prepare_db` from root, and CI's
   `check_generated` expects `.sqlx`/hakari artifacts to be committed and
   fresh (`just hakari` after Cargo.toml changes).
3. `cargo test -p <touched crate>` for each touched crate, from the root,
   with the §4 databases up and `SQLX_OFFLINE` unset.
4. `just check` (or `just check full` for clippy+tsc coverage) is the local
   mirror of CI's format/lint gates.
5. New endpoints: wrap in `packages/sdk` or record as skipped, until
   `just coverage` passes.

Frontend: `bun run check`, `bun run test`, `bun run lint` (CI runs biome with
`--error-on-warnings`).

To verify behavior in the running product, use the local stack (§3) or, on
Cursor Cloud, the `run-app` skill.

## 10. Documented failure modes

| Symptom | Documented fix |
| --- | --- |
| Migration errors persist after `just setup_macrodb` | `just crates/macro_db_client/force_drop_db`, then `just setup_macrodb` (root CLAUDE.md) |
| sqlx "no cached data" in build/tests | `just prepare_db` from root (`--tests` if in test code); never set `SQLX_OFFLINE=true` for tests |
| "type annotations needed" errors in sqlx macros during tests | Caused by running tests offline — unset `SQLX_OFFLINE`, re-run |
| `nix develop` fails | Enable `experimental-features = nix-command flakes` (one-shot flags shown in §1) |
| App loads but API calls return HTML / login fails (macOS) | Host port hijacked (commonly 8080 by macOS WebDriver, 8090 by another dev server). `just doctor-local` shows busy ports; run with `--instance <name> --port-base <free base>` and reuse those flags everywhere |
| Changes to `sync_service` / `lexical_service` / `websocket_service` not visible | These are Docker-built aux images, stale by default — restart with `just run_local --build-aux-services` |
| Seed CLI hits the wrong database | Instance started with explicit `--port-base` must be seeded with the same `--instance` and `--port-base` flags |
| Stale seeded login links after changing ports | Re-run `just seed-scenario apply` to reprint links for the new window |
| `just check` reports almost nothing in CI/shallow clones | No merge-base with `origin/main`; set `CHECK_BASE=<ref>` |
| `agent_harness_service` restart loop in the local stack | Expected when AI provider keys are missing; supply them via Doppler (`DOPPLER_TOKEN`) or ignore |
| Stack cleanup problems on next start | Caused by closing the terminal instead of pressing `q` in `run_local` |
| iOS app loads then JS silently stops | Module Web Worker constructed at module-load time deadlocks WKWebView — construct workers lazily (`apps/web/AGENTS.md` has the full diagnosis recipe) |
