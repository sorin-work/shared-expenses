CREATE TABLE groups (
    id          UUID PRIMARY KEY,
    name        TEXT NOT NULL,
    currency    CHAR(3) NOT NULL,
    invite_code TEXT NOT NULL UNIQUE,
    created_by  UUID NOT NULL REFERENCES users (id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- A member is a person in a group. user_id is NULL for placeholder people
-- who don't have an account (yet). Members with expenses can't be deleted,
-- so leaving a group sets left_at instead.
CREATE TABLE members (
    id           UUID PRIMARY KEY,
    group_id     UUID NOT NULL REFERENCES groups (id) ON DELETE CASCADE,
    user_id      UUID REFERENCES users (id) ON DELETE SET NULL,
    display_name TEXT NOT NULL,
    role         TEXT NOT NULL DEFAULT 'member' CHECK (role IN ('owner', 'member')),
    joined_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    left_at      TIMESTAMPTZ
);

CREATE UNIQUE INDEX members_group_user_key ON members (group_id, user_id) WHERE user_id IS NOT NULL;
CREATE INDEX members_user_id_idx ON members (user_id);
