-- Contract summary table for efficient listing of contracts with event counts.
-- Maintains denormalized stats (event_count, first_seen_ledger, last_seen_ledger)
-- updated on each event insertion to avoid expensive GROUP BY on the events table.

CREATE TABLE IF NOT EXISTS contract_summaries (
    contract_id         TEXT        PRIMARY KEY,
    event_count         BIGINT      NOT NULL DEFAULT 0,
    first_seen_ledger   BIGINT,
    last_seen_ledger    BIGINT,
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Trigger to update contract_summaries on new events.
-- This keeps event counts exact and cheap without relying on full table scans.
CREATE OR REPLACE FUNCTION update_contract_summary()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO contract_summaries (
        contract_id,
        event_count,
        first_seen_ledger,
        last_seen_ledger,
        updated_at
    ) VALUES (
        NEW.contract_id,
        1,
        NEW.ledger,
        NEW.ledger,
        now()
    )
    ON CONFLICT (contract_id) DO UPDATE SET
        event_count = contract_summaries.event_count + 1,
        last_seen_ledger = GREATEST(contract_summaries.last_seen_ledger, EXCLUDED.last_seen_ledger),
        updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_update_contract_summary
AFTER INSERT ON events
FOR EACH ROW
EXECUTE FUNCTION update_contract_summary();

-- Backfill contract_summaries from existing events.
-- This is safe to run multiple times (upsert semantics).
INSERT INTO contract_summaries (
    contract_id,
    event_count,
    first_seen_ledger,
    last_seen_ledger,
    updated_at
)
SELECT
    contract_id,
    count(*)::bigint                  AS event_count,
    min(ledger)                       AS first_seen_ledger,
    max(ledger)                       AS last_seen_ledger,
    now()                             AS updated_at
FROM events
GROUP BY contract_id
ON CONFLICT (contract_id) DO UPDATE SET
    event_count = EXCLUDED.event_count,
    first_seen_ledger = LEAST(
        contract_summaries.first_seen_ledger,
        EXCLUDED.first_seen_ledger
    ),
    last_seen_ledger = GREATEST(
        contract_summaries.last_seen_ledger,
        EXCLUDED.last_seen_ledger
    ),
    updated_at = now();
