# shared-expenses

Split costs fairly with flatmates, partners and travel groups. Create a group,
invite people with a link (or add the ones who don't use the app as
placeholders), record who paid for what, and let the app work out who owes
whom and the fewest payments that settle everything.

## Features

- **Groups** with one currency each (20 currencies, including zero-decimal
  ones like JPY). The creator is the owner and can rename the group, change
  the currency while there are no expenses yet, and delete it.
- **Members**: join through an invite link, or get added as a placeholder
  person without an account. When a placeholder later joins, they can claim
  their place and keep their expenses. Members with history are marked as
  having left instead of being deleted, and only once they are settled up.
- **Expenses** paid by one member and split
  - equally,
  - by exact amounts,
  - by percentages (two decimals), or
  - by shares (e.g. 2 shares for a couple, 1 for a single).

  All money is integer minor units (cents). Amounts that don't divide evenly
  are distributed with the largest remainder method, ties going to members in
  the order they joined, so the same expense always splits the same way.
- **Categories**, search, filters by category and person, and a monthly
  list.
- **Edit history**: every change stores a snapshot; the expense page shows
  who changed what, field by field. Deleted expenses keep their history.
- **Balances** per member and **settle up** suggestions that use the
  minimum number of transfers (exact for groups of up to 16 people with an
  open balance, greedy beyond that).
- **Settlements**: record payments between members (from a suggestion or by
  hand) and undo them.
- **Activity feed** for each group.
- **CSV export** with one column per member showing how each row changed
  their balance, and a balance line at the end. Cells that would be run as
  spreadsheet formulas are escaped.
- Accounts with email and password, profile and password changes.

## Stack

| Part     | Tech                                                                    |
| -------- | ----------------------------------------------------------------------- |
| API      | Rust, axum 0.8, tokio, sqlx 0.8 (PostgreSQL), argon2, JWT, tracing      |
| Web      | SvelteKit 2 (Svelte 5, static adapter), TypeScript                      |
| Database | PostgreSQL 17, migrations embedded in the API binary and run on start   |
| Tests    | `cargo test` (unit + axum handler tests), Vitest                        |
| Tooling  | clippy, rustfmt, ESLint, Prettier, svelte-check, GitHub Actions         |
| Delivery | Docker Compose: Postgres, the API, nginx serving the web app            |

## Running with Docker

```bash
cp .env.example .env   # optional, the defaults work
docker compose up --build
```

Then open http://localhost:5187 and sign in with the demo account:

- **Email:** `demo@example.com`
- **Password:** `demo-password`

The demo user (Alex Morgan) gets three groups: a four person flat share with
four months of rent, bills and groceries, a long weekend in Lisbon using every
split method, and a household shared 60/40 with a partner. The other people in
those groups can sign in too (`maya@example.com`, `jonas@example.com`,
`sam@example.com`, same password), which is handy for trying invites and
placeholders. Set `SEED_DEMO=false` to start with an empty database.

| Service | Host port             | Notes                                               |
| ------- | --------------------- | --------------------------------------------------- |
| web     | http://localhost:5187 | nginx, serves the app and proxies `/api` to the API |
| api     | http://localhost:4407 | `GET /healthz` for health checks                    |
| db      | `localhost:5447`      | user, password and database `shared_expenses`       |

Ports can be changed with `WEB_PORT`, `API_PORT` and `DB_PORT`. Data lives in
the `db-data` volume; `docker compose down -v` removes it.

## Running locally

You need Rust (stable), Node 22+ and a Postgres database. The easiest way is
to run just the database in Docker:

```bash
docker compose up -d db
```

API (listens on :4407 and applies migrations on start):

```bash
cd api
export DATABASE_URL=postgres://shared_expenses:shared_expenses@localhost:5447/shared_expenses
export JWT_SECRET=local-development-secret-change-me-0123456789
export SEED_DEMO=true
cargo run
```

Web (http://localhost:5187, proxies `/api` to the API):

```bash
cd web
npm install
npm run dev
```

Set `API_URL` if the API is not on `http://localhost:4407`.

## Tests and linting

API:

```bash
cd api
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

The split, balance and settle up logic, CSV writer, config and validation
have unit tests. `tests/http.rs` exercises routing, auth and validation
through the axum router without a database.

Web:

```bash
cd web
npm run lint    # prettier + eslint
npm run check   # svelte-check / TypeScript
npm test        # vitest
```

CI runs all of the above, with a Postgres service for the database tests,
and builds the Docker images.

## Architecture

```
api/
  migrations/        SQL migrations, embedded with sqlx::migrate!
  src/domain/        pure logic: split, balance, simplify, currency, money
  src/routes/        axum handlers, one module per resource
  src/auth/          argon2 password hashing, JWT, the AuthUser extractor
  src/seed.rs        demo data
  tests/             handler tests (tower::ServiceExt::oneshot)
web/
  src/lib/           API client, money/split/date helpers, components
  src/routes/        pages (login, groups, expenses, balances, activity...)
```

- The API is a JSON API under `/api`. Errors always look like
  `{"error": {"code", "message", "fields"?}}`, with per-field messages for
  validation errors.
- Every group-scoped endpoint checks membership first and answers 404 for
  groups the user isn't in.
- Balances are computed from the live expenses and settlements on request;
  they are never stored, so they can't drift.
- Expenses and settlements are soft deleted. Each expense change writes a
  JSON snapshot to `expense_revisions` and an entry to `activity`.
- The web app is a single page app built with the static adapter; nginx
  serves it and proxies `/api`, so the browser only talks to one origin.

### API overview

| Method               | Path                                              |                                   |
| -------------------- | ------------------------------------------------- | --------------------------------- |
| POST                 | `/api/auth/signup`, `/api/auth/login`             | returns `{token, user}`           |
| GET, PATCH           | `/api/auth/me`                                    | profile                           |
| POST                 | `/api/auth/password`                              | change password                   |
| GET, POST            | `/api/groups`                                     | my groups with my balance / create |
| GET, PATCH, DELETE   | `/api/groups/{id}`                                | group with members                |
| POST                 | `/api/groups/{id}/invite`                         | new invite code                   |
| POST, PATCH, DELETE  | `/api/groups/{id}/members[/{member}]`             | placeholders, rename, remove/leave |
| GET, POST            | `/api/groups/{id}/expenses`                       | list (`q`, `category`, `member`, `limit`, `offset`) / create |
| GET, PUT, DELETE     | `/api/groups/{id}/expenses/{expense}`             |                                   |
| GET                  | `/api/groups/{id}/expenses/{expense}/history`     | revisions                         |
| GET, POST, DELETE    | `/api/groups/{id}/settlements[/{settlement}]`     |                                   |
| GET                  | `/api/groups/{id}/balances`                       | balances and suggestions          |
| GET                  | `/api/groups/{id}/activity`                       | feed (`before`, `limit`)          |
| GET                  | `/api/groups/{id}/export.csv`                     | CSV export                        |
| GET, POST            | `/api/invites/{code}`, `/api/invites/{code}/join` | preview / join (optionally claim a placeholder) |

## License

MIT
