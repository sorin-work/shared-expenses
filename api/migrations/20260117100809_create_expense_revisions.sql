-- Every create and edit stores a full snapshot of the expense so the history
-- can be shown even after later edits or a delete.
CREATE TABLE expense_revisions (
    id         UUID PRIMARY KEY,
    expense_id UUID NOT NULL REFERENCES expenses (id) ON DELETE CASCADE,
    revision   INT NOT NULL,
    action     TEXT NOT NULL CHECK (action IN ('created', 'updated', 'deleted')),
    snapshot   JSONB NOT NULL,
    changed_by UUID REFERENCES users (id) ON DELETE SET NULL,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (expense_id, revision)
);
