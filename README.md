# shared-expenses

Split costs fairly with flatmates, partners and travel groups. Create a group,
invite people with a link (or add the ones who don't use the app as
placeholders), record who paid for what, and let the app work out who owes
whom and the fewest payments that settle everything.

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

## License

MIT
