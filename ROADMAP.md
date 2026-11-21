# Roadmap

Things I might add to shared-expenses. The app covers the loop of a shared
budget today (groups, members and placeholders, four split methods, balances,
settle up with the fewest transfers, settlements, history, activity and CSV
export), so nothing here is promised or scheduled. These are ideas, written
down with enough detail that I (or anyone else) can pick one up later without
starting from zero. Items may change, move between groups or be dropped.

What the app does today is in the [README](README.md), including the API
overview and the architecture.

- **Next**: small or well understood. I would start with these.
- **Later**: worth doing, but bigger or waiting on a decision.
- **Maybe**: not sure yet that they are worth the extra complexity.

Sizes are rough: **S** is an evening or two, **M** a few evenings up to a
couple of weekends, **L** several weeks of spare time.

Each item is meant to fit in one pull request and to be covered by the test
suite. There are no browser tests. In the API, pure logic lives in
`api/src/domain` (split, balance, simplify, currency, money) with unit tests
next to the code, `api/tests/http.rs` drives the axum router without a
database, and `api/tests/api_*.rs` run against PostgreSQL when
`TEST_DATABASE_URL` is set (CI always sets it), using the helpers in
`api/tests/common/mod.rs`. In the web app, Vitest covers the modules in
`web/src/lib`; the Svelte pages and components have no tests. So logic worth
testing (money, splits, dates, parsing, diffs) belongs in `api/src/domain` or
a plain module in `web/src/lib`, not inside a handler or a component. Items
that change the database bring a new SQL file under `api/migrations`, which
the API embeds and runs on start. File names below are relative to the
repository root.

When an item ships, its Overview cell says `Shipped`, its section moves under
a `## Shipped` heading at the end and shrinks to one paragraph on what was
built and where, and the same pull request updates the README (feature list,
configuration and API overview) and adds a line to the `Unreleased` section
of `CHANGELOG.md` (the first item to ship creates that file).

"Depends on" lists real prerequisites only. Items that merely touch the same
file are not listed there; the bigger overlaps are mentioned in the items.

## Overview

| #   | Item                                         | Group | Area          | Size | Depends on |
| --- | -------------------------------------------- | ----- | ------------- | ---- | ---------- |
| 1   | Signing out that really signs out            | Next  | auth          | M    | -          |
| 2   | Limits on sign in and sign up                | Next  | auth          | S    | -          |
| 3   | Security headers for the web app             | Next  | security      | S    | -          |
| 4   | Handing over a group                         | Next  | groups        | S    | -          |
| 5   | Edit conflicts and double submits            | Next  | expenses      | S    | -          |
| 6   | Backups and restore                          | Later | ops           | M    | -          |
| 7   | Email: password reset and address changes    | Later | accounts      | M    | -          |
| 8   | Deleting an account                          | Later | accounts      | M    | 4          |
| 9   | Placeholder claims the group agrees with     | Later | members       | S    | -          |
| 10  | Invite links that expire                     | Later | members       | S    | -          |
| 11  | Several payers for one expense               | Later | expenses      | M    | -          |
| 12  | Itemised bills                               | Later | expenses      | M    | -          |
| 13  | Refunds and money received                   | Later | expenses      | S    | -          |
| 14  | Recurring expenses                           | Later | expenses      | M    | -          |
| 15  | Expenses in another currency                 | Later | currency      | L    | -          |
| 16  | Receipt photos                               | Later | expenses      | M    | -          |
| 17  | Comments on expenses                         | Later | expenses      | S    | -          |
| 18  | A default split for the group                | Later | expenses      | S    | -          |
| 19  | Group categories                             | Later | categories    | S    | -          |
| 20  | Spending summaries                           | Later | insights      | M    | -          |
| 21  | All my balances in one place                 | Later | overview      | S    | -          |
| 22  | How to pay me                                | Later | settlements   | S    | -          |
| 23  | Email notifications and reminders            | Later | notifications | M    | 7          |
| 24  | Closing a group                              | Later | groups        | S    | -          |
| 25  | Who may change what                          | Later | groups        | M    | -          |
| 26  | Moving groups in and out                     | Later | import/export | M    | -          |
| 27  | Tests for the web pages                      | Later | tests         | M    | -          |
| 28  | Accessibility pass                           | Later | web           | M    | 27         |
| 29  | Running a private instance                   | Later | ops           | S    | 2          |
| 30  | Large groups                                 | Maybe | performance   | M    | -          |
| 31  | Adding expenses offline                      | Maybe | web           | L    | 5          |
| 32  | Translations and local formats               | Maybe | web           | L    | -          |
| 33  | Two-step sign in                             | Maybe | auth          | M    | 1          |
| 34  | Metrics for monitoring                       | Maybe | ops           | S    | -          |
| 35  | Personal API tokens and a described API      | Maybe | api           | M    | 1          |
| 36  | Importing a bank statement                   | Maybe | import/export | L    | 26         |

## Next

### 1. Signing out that really signs out

A login is a JWT signed with `JWT_SECRET` and valid for `TOKEN_TTL_HOURS`
(a week by default). `TokenKeys::verify` in `api/src/auth/token.rs` checks only
the signature and the expiry, so nothing can end a token early: "Sign out"
in the web app only removes it from `localStorage`
(`web/src/lib/session.svelte.ts`), changing the password
(`POST /api/auth/password`) leaves every other login working, and a token
copied off a shared computer stays valid for the rest of the week. Rotating
`JWT_SECRET` signs out everyone, which is the only lever today.

- A `sessions` table: id, user, created, last seen, user agent, and
  `revoked_at`. The token carries the session id (`sid` claim) next to `sub`.
- `AuthUser::from_request_parts` (`api/src/auth/extract.rs`) checks the session
  after the signature: one indexed lookup per request, which is cheap next to
  the queries every handler already runs.
- `POST /api/auth/logout` revokes the current session; the web app calls it
  before clearing `localStorage`. Changing the password revokes all other
  sessions of the user.
- A list of signed-in devices on the settings page
  (`web/src/routes/settings/+page.svelte`) with "sign out" per device and
  "sign out everywhere else".
- Tricky: `last_seen_at` on every request is a write per call. Update it at
  most every few minutes.
- Tricky: the token stays in `localStorage`, where any script on the page can
  read it. Moving to an `HttpOnly` cookie would be better against theft but
  needs CSRF protection on every write; with item 3's CSP the risk shrinks, and
  revocable sessions limit the damage. I lean towards keeping the bearer token
  for now and revisiting with item 35.
- Tricky: tests sign up and use the token right away (`TestApp::signup` in
  `api/tests/common/mod.rs`); they keep working as long as signup creates a
  session.
- Tests: unit tests for the new claim; database tests in
  `api/tests/api_auth.rs`: a revoked session gets 401, a password change
  revokes the others but not the current one, logout works once.

### 2. Limits on sign in and sign up

`/api/auth/login` and `/api/auth/signup` can be called as fast as a client
likes. Every login runs argon2 (even for unknown emails, on purpose, through
`DUMMY_HASH`), so a burst of requests is both a password guessing attack and
an easy way to keep the CPU busy. Signup also tells whether an email is
registered ("is already registered").

- A per-IP and per-email limit on login (say 10 a minute per IP and 5 failures
  per 15 minutes per email), and a per-IP limit on signup, with `429` and
  `Retry-After` in the usual error shape.
- `tower_governor` gives a per-IP token bucket as a layer; the per-email
  failure count is a small in-memory map with expiry, which is enough for one
  API process. With several processes it would need the database, which this
  app does not need today.
- The client IP comes from nginx (`X-Forwarded-For` in `web/nginx.conf`). Trust
  that header only when the request comes from the web container, or anyone can
  pick their own IP; item 29 covers the published API port.
- A limit on concurrent argon2 hashes (a semaphore around
  `password::verify_async`) so a burst cannot starve the blocking thread pool.
- Tricky: the web app shows field errors from the error shape; a 429 needs a
  plain message on the login and signup pages (`web/src/lib/errors.ts`).
- Undecided: answer signup for a taken email the same as a new one and send a
  mail instead. That needs mail (item 7); until then the message stays.
- Tests: router tests in `api/tests/http.rs` for the 429 and its header with a
  low limit from config; a unit test for the failure map's expiry.

### 3. Security headers for the web app

`web/nginx.conf` serves the app with no security headers at all: no
Content-Security-Policy, no `X-Frame-Options`, no `Referrer-Policy`. The login
token sits in `localStorage`, so one injected script is a stolen account, and
the app can be framed by any page.

- In `nginx.conf`: a CSP that fits a static SvelteKit build (`default-src
  'self'`, `script-src 'self'` plus the hashes SvelteKit's inline start script
  needs, `style-src 'self' 'unsafe-inline'` if Svelte's transitions require it,
  `connect-src 'self'`, `frame-ancestors 'none'`, `base-uri 'self'`,
  `form-action 'self'`), `X-Content-Type-Options: nosniff`, `Referrer-Policy:
  same-origin` and `Permissions-Policy` turning off what the app never uses.
- SvelteKit can produce the CSP itself (`kit.csp` in `svelte.config.js` with
  `mode: 'hash'`), which writes the right hashes for the inline bootstrap
  script into a `<meta>` tag at build time. That is the reliable way; nginx
  adds the headers that cannot live in a meta tag (`frame-ancestors`).
- The API answers JSON only; `nosniff` and `Cache-Control: no-store` on
  authenticated responses are enough there, as a tower layer in
  `api/src/routes/mod.rs`.
- Tricky: `add_header` in nginx is not inherited into a `location` that has
  its own `add_header` (the `/_app/immutable/` and `/` blocks both set
  `Cache-Control`). Put the headers in an `include` file used in each location,
  or they silently vanish.
- Tricky: `npm run dev` does not go through nginx. The meta tag CSP applies in
  dev too, which is good for catching problems early.
- Tests: a router test that API responses carry the headers; for nginx, a
  check in CI after `docker compose build` that runs the web image and asserts
  the headers with `curl -I`.

### 4. Handing over a group

The owner cannot leave a group ("the owner can't leave the group; delete the
group instead" in `api/src/routes/members.rs`), and there is no way to make
someone else the owner. A flat share outlives its first tenant; today the
person who created the group has to stay in it forever or delete everyone's
history.

- `POST /api/groups/{id}/owner` with a member id: the current owner hands over,
  in one transaction that sets the new owner and makes the old one a member.
  Only to a member with an account and who has not left.
- After that the old owner can leave like anyone else, once settled up.
- The members page (`web/src/routes/groups/[id]/members/+page.svelte`) gets
  "Make owner" next to each eligible member, with a confirmation.
- Activity entry `owner_changed` with both names, shown in the feed
  (`web/src/lib/activity.ts` maps kinds to text).
- Tricky: `groups.created_by` is not the owner; the role on `members` is. Leave
  `created_by` alone as history.
- Undecided: several owners. Simple to allow (the role is per member), but
  then "the owner can't leave" needs "the last owner". I lean towards one owner
  now; item 25 is where roles grow.
- Tests: database tests in `api/tests/api_groups.rs`: hand over, old owner
  leaves, a non-owner cannot hand over, a placeholder cannot become owner.

### 5. Edit conflicts and double submits

Two people fixing the same expense at the same time: `update_expense` locks the
row so the revisions are numbered in order, but the second save simply
overwrites the first, and its author never sees what changed. And a double tap
on "Save" on a slow phone creates the expense twice, because
`POST /api/groups/{id}/expenses` has no way to know it is a retry.

- Conflicts: the expense view gets its current `revision`. `PUT` takes the
  revision it was based on; when it is not the latest, answer `409` with the
  current expense, and the form shows "Maya changed this meanwhile" with the
  field-by-field difference (`web/src/lib/history.ts` already renders such
  diffs).
- Double submits: the client sends an `Idempotency-Key` header (a UUID per
  open form) on create. The API stores the key with the created expense id in a
  small table for a day and returns the same expense for a repeated key.
- The same key for settlements, where a double tap records a payment twice.
- Tricky: the idempotency row has to be written in the same transaction as the
  expense, with a unique constraint on (user, key), so two parallel requests
  cannot both create one.
- Tricky: an old client without the revision field. Treat a missing revision
  as "no check" for one release, then require it.
- Tests: database tests for a stale revision getting 409, the same key giving
  one expense under parallel requests (`tokio::join!`), and a key reused by
  another user not leaking the first user's expense.
