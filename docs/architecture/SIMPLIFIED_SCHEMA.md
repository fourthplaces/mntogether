# Simplified Schema

> **SUPERSEDED — 2026-04-22.** This document described a pre-pivot three-type post system (`service`, `opportunity`, `business`) and an org table with columns (`organization_type`, `claim_token`, etc.) that no longer exist. It predates the nine-type post model, the per-type field-group architecture, and the organisation_links split.
>
> **Current authoritative source:** [`DATA_MODEL.md`](DATA_MODEL.md).
>
> The only idea here worth preserving is the framing principle — *don't create a column for every attribute; use `description` plus tags plus targeted fields*. That principle is still alive in the post-pivot design; see [`DATA_MODEL.md` §11 "What Editorial does NOT store"](DATA_MODEL.md#11-what-editorial-does-not-store) for the current version.
