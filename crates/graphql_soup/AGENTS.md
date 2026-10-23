# graphql_soup

## What it is

Backs the Soup feature (the app's unified entity search/home surface): the
soup-domain GraphQL wrapper types. The composed schema (root query, viewer,
SDL export) lives in `complete_graph` — see `../complete_graph/AGENTS.md` for
the schema conventions (presence-of-`id` normalized-cache rules and the
viewer pattern), which apply to every type defined here.

## Gotchas

- `SoupItemLoader` (single-entity lookups, `soupUpdates` hydration) batches
  one filtered Soup request per user. Entity types of removed features (email
  threads, calendar events, CRM) are not represented in Soup, so asking for
  one is a loader error rather than an empty hydration.
- `NOTIFIED_AT` maps to Soup's notified request, not a simple sort: `soup`
  accepts it descending only and `groupSoup` rejects it. Its continuation
  cursors carry a different sort tag, so `inputs.rs` decodes both cursor shapes.
