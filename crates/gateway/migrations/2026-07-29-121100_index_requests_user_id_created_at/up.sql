-- Speeds up "latest request per user" lookups (admin/dashboard LATERAL joins)
-- and the requests.user_id foreign-key checks. Without it, a query like
--   SELECT ... FROM requests WHERE user_id = $1 ORDER BY created_at DESC LIMIT 1
-- degrades into a sequential scan + sort over the whole (multi-million-row)
-- table, pegging a CPU core and blocking concurrent INSERTs.
--
-- CONCURRENTLY avoids taking a lock that would block INSERTs while the index
-- builds; it cannot run inside a transaction (see metadata.toml).
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_requests_user_id_created_at
    ON requests (user_id, created_at DESC);
