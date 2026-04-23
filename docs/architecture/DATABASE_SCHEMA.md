# Database Schema Architecture

> **SUPERSEDED — 2026-04-22.** This file used to enumerate every table and column at a point in time. It fell out of sync within a few migrations (last comprehensive update was through migration `000171`; schema is now past `000237`), and re-syncing it by hand proved unsustainable.
>
> **Current authoritative sources:**
>
> | For | Look at |
> |---|---|
> | Living narrative of the data model | [`DATA_MODEL.md`](DATA_MODEL.md) |
> | Post-table shape (exact Rust struct) | `packages/server/src/domains/posts/models/post.rs` |
> | Post-type field-group requirements | [`POST_TYPE_SYSTEM.md`](POST_TYPE_SYSTEM.md) |
> | Tag vocabulary | [handoff `TAG_VOCABULARY.md`](../handoff-root-signal/TAG_VOCABULARY.md) |
> | Edition lifecycle | [`EDITION_STATUS_MODEL.md`](EDITION_STATUS_MODEL.md) |
> | Actual SQL | `packages/server/migrations/` (append-only log; read models if you want the current design — see [`CLAUDE.md` §Reading This Codebase](../../CLAUDE.md)) |
>
> If you arrived here from a cross-link, treat that link as stale and update it when you're done.
