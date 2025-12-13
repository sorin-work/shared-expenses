CREATE TABLE settlements (
    id           UUID PRIMARY KEY,
    group_id     UUID NOT NULL REFERENCES groups (id) ON DELETE CASCADE,
    from_member  UUID NOT NULL REFERENCES members (id),
    to_member    UUID NOT NULL REFERENCES members (id),
    amount_cents BIGINT NOT NULL CHECK (amount_cents > 0),
    settled_on   DATE NOT NULL,
    note         TEXT,
    created_by   UUID NOT NULL REFERENCES users (id),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at   TIMESTAMPTZ,
    CHECK (from_member <> to_member)
);
