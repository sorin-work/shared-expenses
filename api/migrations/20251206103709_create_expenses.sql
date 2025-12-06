CREATE TABLE expenses (
    id           UUID PRIMARY KEY,
    group_id     UUID NOT NULL REFERENCES groups (id) ON DELETE CASCADE,
    description  TEXT NOT NULL,
    amount_cents BIGINT NOT NULL CHECK (amount_cents > 0),
    paid_by      UUID NOT NULL REFERENCES members (id),
    split_method TEXT NOT NULL CHECK (split_method IN ('equal', 'exact', 'percent', 'shares')),
    category     TEXT NOT NULL DEFAULT 'general' CHECK (category IN (
                     'general', 'groceries', 'rent', 'utilities', 'dining', 'transport', 'travel',
                     'accommodation', 'entertainment', 'household', 'gifts', 'health')),
    spent_on     DATE NOT NULL,
    notes        TEXT,
    created_by   UUID NOT NULL REFERENCES users (id),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at   TIMESTAMPTZ
);

-- One row per participant. `input` is what the user typed for the split
-- method (exact cents, basis points or a share count) so the form can be
-- shown again exactly as it was entered.
CREATE TABLE expense_shares (
    expense_id   UUID NOT NULL REFERENCES expenses (id) ON DELETE CASCADE,
    member_id    UUID NOT NULL REFERENCES members (id),
    input        BIGINT NOT NULL,
    amount_cents BIGINT NOT NULL CHECK (amount_cents >= 0),
    PRIMARY KEY (expense_id, member_id)
);
