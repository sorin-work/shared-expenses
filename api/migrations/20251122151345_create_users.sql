CREATE TABLE users (
    id            UUID PRIMARY KEY,
    email         TEXT NOT NULL,
    name          TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Emails are compared case-insensitively.
CREATE UNIQUE INDEX users_email_key ON users (lower(email));
