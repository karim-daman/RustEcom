-- Migration: create discounts table

CREATE TABLE IF NOT EXISTS discounts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    percent INTEGER NOT NULL,
    category_id TEXT NULL,
    min_price_cents INTEGER NULL,
    max_price_cents INTEGER NULL,
    active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

-- Index for quick category lookups
CREATE INDEX IF NOT EXISTS idx_discounts_category_id ON discounts(category_id);

-- Index for price-based lookups
CREATE INDEX IF NOT EXISTS idx_discounts_price ON discounts(min_price_cents, max_price_cents);
