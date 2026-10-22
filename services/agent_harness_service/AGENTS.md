# agent_harness_service

## What it is

Part of the agent sessions feature — the composition root for in-process
agent sessions: agent-session trigger events in, in-process ACP runtime
sessions out.

## Start here

- `src/main.rs`: composition root — builds Postgres repositories, the
  in-memory runtime manager, channel/session services, and wires
  `macro.channels`-derived triggers into `macro.agent_sessions`.
- `src/api.rs`: the full agent-session HTTP API (durable metadata/log reads
  plus live in-memory transport operations), composed from each domain
  crate's own router (`agent_session`, `agent_egress`, `agent_harness`).
- `src/model_providers/`: in-memory runtime model discovery (`in_memory.rs`).
- `src/containers.rs`: in-process agent session transport attachment.
- `src/trigger.rs`: turns channel/webhook triggers into sessions.
- `src/internal_mcp.rs`: the internal MCP server exposed to running sessions.

## Scope

- Owns: the deployed process wiring every agent-session domain crate
  together — not the domain logic itself, which lives in `crates/agent_harness`,
  `crates/agent_session`, `crates/agent_egress`, `crates/folds/agent_fold`,
  `crates/agent_inmem`, `crates/agent_trigger`.

## Recipes

- Add a new agent-session route: the owning domain crate's `inbound::axum_router`
  → mount it in `src/api.rs`.
- Inspect a session's protocol log: `just session-log <channel_id>`.

## Tool contracts

- Internal session tools require a verified acting user plus owner/edit authorization. Keep their restricted prompt/stop route separate from human approval controls.

## Checks

- `cargo test -p agent_harness_service` (`SQLX_OFFLINE` unset).
- `just check` (`SQLX_OFFLINE=true cargo check`).

Code-mode credentials sign the authenticated session bot identity for SDK
message attribution after owner approval, while retaining user authentication.

## Related

- Rules: `.agents/rules/rust/architecture.md`, `.agents/rules/config-and-service-urls.md`
