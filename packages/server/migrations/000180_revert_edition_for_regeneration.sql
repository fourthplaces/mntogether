-- Revert the Yellow Medicine County published edition back to draft
-- so it can be regenerated with weight-aware post template assignments.
--
-- The layout engine now assigns templates based on slot weight:
--   heavy  → feature / feature-reversed
--   medium → gazette / bulletin
--   light  → digest / ticker / ledger
--
-- Previously all slots were assigned 'gazette' regardless of weight.
-- After running this migration, regenerate + republish via the admin API.

UPDATE editions
SET status = 'draft'
WHERE id = 'a48175a0-4f1e-460a-a39a-fae216bf75ae'
  AND status = 'published';
