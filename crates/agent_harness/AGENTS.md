# agent_harness

## What it is

Part of agent sessions: orchestrates built-in in-memory Macro agent sessions end to
end — driving `agent_inmem`, routing triggers to a session, and managing ACP traffic
for the in-process agent.

## Start here

- `src/domain/service.rs` + `src/domain/ports.rs`: the harness service and
  the ports for in-memory agent lifecycle and containers.
- `src/domain/trigger_router.rs`: routes an incoming trigger to a session.
- `src/inbound/kafka.rs`: trigger ingestion.

## Scope

- Owns: session lifecycle orchestration for in-memory agent sessions, trigger
  routing, tool/MCP approval wiring.
- Not here: session state/persistence → `agent_session`; credential
  stamping for egress → `agent_egress`; the ACP wire protocol →
  `agent_runtime_protocol`; the in-memory agent loop itself → `agent_inmem`.

## Boundaries

- Depends on: `agent_session`, `agent_egress`, `agent_runtime_protocol`,
  `agent_trigger`, `agent_inmem`, `ai_billing`.
- Used by: `agent_harness_service` (its sole host).

## Checks

- `cargo test -p agent_harness` (`SQLX_OFFLINE` unset).

## Related

- Rules: `.agents/rules/rust/architecture.md`, `.agents/rules/rust/security.md`
