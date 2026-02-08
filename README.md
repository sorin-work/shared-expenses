# shared-expenses

Split costs fairly with flatmates, partners and travel groups. Create a group,
invite people with a link (or add the ones who don't use the app as
placeholders), record who paid for what, and let the app work out who owes
whom and the fewest payments that settle everything.

## Running with Docker

```bash
cp .env.example .env   # optional, the defaults work
docker compose up --build
```

Then open http://localhost:5187 and create an account.

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

## License

MIT
