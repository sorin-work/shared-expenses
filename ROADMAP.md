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

## Later

### 6. Backups and restore

All data is in the `db-data` volume and the README only says that `docker
compose down -v` removes it. For a group that has tracked a flat's bills for
two years, losing the database means nobody knows who owes whom.

- A `backup` service in `docker-compose.yml` behind a compose profile: once a
  night `pg_dump -Fc` into a bind-mounted folder, named by date, keeping the
  last N (`BACKUP_KEEP`, default 14).
- `docs/operations.md` with restoring (`pg_restore --clean` into a stopped
  stack, then start: migrations run on start and bring an old dump forward),
  upgrading, and where the logs go (`LOG_FORMAT=json`).
- Tricky: migrations are embedded and run on start (`MIGRATOR` in
  `api/src/lib.rs`). A dump from a newer version restored under an older binary
  fails with an unknown migration; the docs should say to restore with the
  version that made the dump, then upgrade.
- Tricky: receipts (item 16) would add files; the backup must then include
  that volume too.
- Tests: CI can dump the seeded database in the existing Postgres service,
  restore it into a second database and run the database tests' seed checks
  against it.

### 7. Email: password reset and address changes

The app sends no mail at all. A forgotten password means a lost account (the
demo hint aside), and the email address cannot be changed: `PATCH
/api/auth/me` only updates the name.

- Outgoing mail with `lettre` over SMTP, configured with `SMTP_URL` and
  `MAIL_FROM`, off when unset. A `mailpit` service in `docker-compose.yml` catches
  the mail in development.
- Password reset: `POST /api/auth/password-reset` with an email always answers
  204; when the account exists it mails a link with a random token stored
  hashed, valid for an hour, single use. `POST
  /api/auth/password-reset/{token}` sets the new password and revokes all
  sessions (item 1, if landed).
- Email change: needs the current password, mails a confirmation link to the
  new address and a notice to the old one, and switches when confirmed.
- Web pages: "Forgot password?" on the login page, and the two token pages as
  SvelteKit routes. The static adapter serves them as client routes, so the
  links point at the web app, built from a `PUBLIC_URL` setting.
- Tricky: without SMTP the reset form must say that this instance cannot send
  mail, instead of pretending a mail is on its way.
- Tricky: rate limits on the reset request per IP and per address, or it sends
  mail to anyone (item 2's limiter can be reused).
- Tests: database tests with a mail transport that collects messages: the
  token works once, expires, does not reveal unknown addresses; email change
  needs the password and switches only after confirmation.

### 8. Deleting an account

There is no way to delete an account, and the schema would refuse it anyway:
`groups.created_by`, `expenses.created_by` and `settlements.created_by`
reference `users` without an `ON DELETE` action. For an app holding who paid
whom, people expect to be able to leave completely.

- `DELETE /api/auth/me` with the password. Refused while the user owns a group
  with other members (hand it over first, item 4) or has an open balance in any
  group, with a list of what is in the way.
- What happens to the person's history: the member rows stay so every group's
  balances still add up, but they turn into placeholders: `user_id` set to null
  (the foreign key already does `ON DELETE SET NULL`) and `left_at` set. Groups
  where the user was alone are deleted.
- A migration changes the `created_by` foreign keys to `ON DELETE SET NULL` and
  makes the columns nullable; `expense_revisions.changed_by` and
  `activity.actor_id` already are.
- Tricky: the views show the creator's name from the user row today. Without a
  user they need a fallback ("a former member").
- Tricky: snapshots in `expense_revisions` and payloads in `activity` contain
  display names. Those are the group's shared record; I would keep them and say
  so on the delete page.
- Depends on item 4, or owners could never delete.
- Tests: database tests for each refusal, a successful delete keeping the
  group's balances unchanged, and a sole-member group disappearing.

### 9. Placeholder claims the group agrees with

When someone joins through the invite link they can claim any unclaimed
placeholder (`claim_member_id` in `POST /api/invites/{code}/join`), and with it
that person's whole history and balance. The invite link is often posted in a
group chat; anyone in the chat can claim "Maya" before Maya does, by mistake
or not, and nobody is asked.

- A claim becomes a request: the joiner is a new member at once, and the claim
  waits for the owner (or any existing member with an account) to confirm. On
  confirm, the placeholder's history moves to the new member and the empty new
  row is removed, in one transaction.
- Alternatively, a personal link per placeholder ("send this to Maya"), which
  claims without asking. Both ways can exist; the personal link is the easy one
  for the owner.
- The members page shows pending claims with "Confirm" and "Decline".
- Tricky: moving history. Expenses (`paid_by`), shares and settlements point at
  the placeholder's member id; merging two member rows means updating all three
  and deleting the newer row, or simpler, giving the placeholder row the user
  and deleting the new one. The second keeps every reference intact.
- Tricky: the member who claims may already have added expenses as their new
  self before the claim is confirmed. Then both rows have history and a merge
  must move the newer row's references to the placeholder.
- Tests: database tests for a pending claim, confirm, decline, the personal
  link, and the merge with history on both rows keeping balances right.

### 10. Invite links that expire

Each group has one `invite_code` that works forever, for anyone, until the
owner regenerates it (`regenerate_invite`). A link posted once in a big chat
two years ago still lets strangers into the flat's finances.

- Invites as rows: code, group, created by, `expires_at` (default 14 days),
  optional `max_uses`, uses, revoked. The old `groups.invite_code` becomes the
  first row, with no expiry, so existing links keep working.
- The members page lists active links with their expiry and uses, with "new
  link" and "revoke".
- `GET /api/invites/{code}` keeps its preview shape; an expired or used up link
  answers 410 with a message the join page (`web/src/routes/join/[code]`) can
  show.
- Tricky: the invite code alphabet and length (`new_invite_code`) are fine; the
  preview endpoint is unauthenticated, so it must be rate limited like login
  (item 2) to make guessing pointless.
- Tests: database tests for expiry, max uses, revoke, and the migrated code
  still working.

### 11. Several payers for one expense

An expense has one payer (`expenses.paid_by`). A restaurant bill paid half by
card and half in cash, or a holiday house paid by two people, has to be entered
as two expenses with the same split, which doubles the list and the chance of
getting one of them wrong.

- An `expense_payments` table (expense, member, amount), with the amounts
  summing to the total. `paid_by` stays for one payer (the common case) or is
  replaced; I lean towards always writing payments and keeping `paid_by` as a
  derived "main payer" for the list.
- `domain/balance.rs` adds each payment instead of one `paid_by` amount;
  `compute_for_group` in `api/src/routes/balances.rs`, the groups list query
  in `groups.rs` (which sums `paid_by` in SQL) and the CSV export
  (`api/src/routes/export.rs`) change with it.
- The form (`ExpenseForm.svelte`) gets "paid by several people" with an amount
  per payer, using the same exact-amount input as the split.
- Tricky: the groups list computes "my balance" in SQL, separately from
  `balance::compute`. Two places that must agree; a test that compares them for
  the seeded groups catches drift. Item 30 may fold them into one.
- Tricky: history snapshots and the revision diff (`changed_fields`) need the
  payments.
- Tests: unit tests for balances with several payers; database tests for create,
  edit and the list balance matching the balances page.

### 12. Itemised bills

A shared supermarket receipt or a restaurant bill is rarely split one way: the
wine was for two people, the kids' meals for one family. Today that means one
expense per group of items, or splitting by exact amounts worked out on a
calculator.

- An itemised split method: a list of items, each with an amount and the
  members who share it (equally), plus tax or tip spread over everyone in
  proportion. The result is an exact split, so balances and the rest of the app
  need no change; the items are kept so the form can be shown again.
- Store items as JSON on the expense or in an `expense_items` table. The table
  is cleaner for history diffs; JSON is enough if items are only for the form.
- The calculation in `domain/split.rs` with the largest remainder rule the
  other methods use, so the same bill always splits the same way.
- Tricky: the CHECK constraint on `split_method` lists four values; the
  migration adds the fifth.
- Tests: unit tests for item totals, shared items, proportional tip and
  rounding; a database test for saving and loading items.

### 13. Refunds and money received

`amount_cents` must be positive (`CHECK (amount_cents > 0)`). A refund from a
shop for a returned item, a deposit coming back from the landlord, or money a
group receives (selling the old sofa) cannot be entered, except as a backwards
settlement, which is wrong when the money comes from outside the group.

- A `kind` on expenses: `expense` (today) or `income`. Income received by one
  member and shared by the others is an expense with the sign flipped in the
  balance calculation.
- The form gets "Money received" next to "Expense"; lists show it in another
  colour with "received by".
- A refund of an existing expense is an income with a link back to it, created
  from the expense page with the same split.
- Tricky: every sum that assumes positive amounts: `balance::compute`, the
  groups list SQL, CSV export, and spending summaries (item 20), which should
  subtract refunds from their category.
- Tests: unit tests for balances with income; database tests for the refund
  flow and the CSV row signs.

### 14. Recurring expenses

Rent, internet, a streaming account and the cleaning service are the same
expense every month. Flat shares enter them by hand and forget one.

- A `recurring_expenses` table: a template (description, amount, payer, split,
  category) and a rule (monthly on day N, weekly, every N months), next date,
  end date, paused.
- A background task in the API (a `tokio` interval started in `main.rs`) that
  creates the due expenses once a day through `insert_expense`, so history and
  activity are written as usual with "added automatically".
- Created expenses can be edited like any other; editing the template changes
  only future ones.
- Tricky: catching up. If the API was down for three days, the task creates
  each missed one, never more than once: a unique constraint on (template,
  date) makes that safe even with two API processes.
- Tricky: day 31 in February. Use the last day of the month.
- Tricky: the members in the split may have left. Skip them and note it, or
  pause the template and say why; I lean towards pausing.
- Tests: unit tests for the date rule; database tests for creation, catching up
  without duplicates, and a left member pausing the template.

### 15. Expenses in another currency

A group has one currency and every expense is in it. Travel groups pay in
several currencies; today someone converts each receipt by hand. Splitwise
keeps this behind its paid plan, which makes it one of the most asked-for
features in open alternatives.

- An expense keeps the currency and amount it was paid in, plus the rate to the
  group currency at that moment, and its converted amount in the group
  currency. Balances and settle up keep working in the group currency only.
- Rates: typed by the payer (the card statement shows the real rate), with an
  optional fetch from a rate source the instance owner configures. No rate
  service is required; the European Central Bank's daily reference rates are a
  free, unauthenticated XML file and a sensible optional default.
- The list and the expense page show both amounts ("¥12,000, €74.10").
- Tricky: rounding. Convert the total once, then split the converted total with
  the usual largest remainder rule, so shares still add up exactly in the group
  currency.
- Tricky: zero-decimal currencies on either side (`minor_units` in
  `domain/currency.rs`). Keep the rate as a decimal string or a scaled integer,
  never a float.
- Tricky: editing the amount later recomputes with the stored rate unless the
  rate is edited too.
- Undecided: settling in another currency ("I paid you back in pounds"). The
  same mechanism works for settlements; I would do expenses first.
- Tests: unit tests for conversion and splitting with zero and two decimal
  currencies; database tests for create, edit and CSV export with both amounts.

### 16. Receipt photos

"Did you really pay 84 euros for that?" is answered with the receipt. Today the
receipt lives in someone's camera roll.

- Upload one or more images (JPEG, PNG, WebP, HEIC converted if a decoder is
  available) or a PDF per expense, stored on disk in a `receipts` volume under
  random names, served only to group members through the API.
- Strip EXIF data (phone photos carry GPS) and downscale large images when
  storing, with the `image` crate.
- Tricky: the API serves JSON with a bearer token, but an `<img>` tag cannot
  send the `Authorization` header. Either fetch the image with the token and
  show it from a blob URL in the web app, or issue short-lived signed URLs for
  images. I lean towards signed URLs that live a few minutes.
- Tricky: deleted expenses are soft deleted and keep their history; their
  receipts should stay until the group is deleted.
- Tricky: upload size and request body limits in axum (`DefaultBodyLimit`) and
  in nginx (`client_max_body_size`, 1 MB by default).
- Tests: database tests for upload, access by a non-member (404), the signed
  URL expiring, and metadata removed from a JPEG fixture.

### 17. Comments on expenses

Questions about an expense ("is this the one for the party?") go to the group
chat and get lost. A short comment thread on the expense keeps them where they
belong. Splitwise has this and people use it.

- An `expense_comments` table (expense, author user, text up to 1000
  characters, created, deleted). Any member can comment; authors can delete
  their own.
- Comments show on the expense page under the history, and a new comment is an
  activity entry.
- Tricky: comments on deleted expenses stay readable with the history.
- Tests: database tests for adding, listing, deleting one's own, refusing
  another's, and a non-member getting 404.

### 18. A default split for the group

The demo household is shared 60/40, and every expense in such a group needs the
percentages typed again. Couples and flat shares with unequal rooms do this
dozens of times a month.

- A default split on the group: method and values per member (for example
  percent 60/40, or shares 2/1/1), set on the settings page by the owner.
- New expenses start from it; the form still allows changing it per expense.
- Tricky: members joining or leaving. A default that names a member who has
  left is invalid; drop them and spread the rest, or fall back to equal with a
  warning on the settings page. I lean towards the warning.
- Tests: database tests for saving and validating the default; a Vitest test for
  the form helper that applies it (`web/src/lib/split.ts`).

### 19. Group categories

Twelve categories are fixed, and the list is written three times: the CHECK
constraint in `api/migrations/20251206103709_create_expenses.sql`,
`CATEGORIES` in `api/src/domain/category.rs` and `CATEGORIES` in
`web/src/lib/categories.ts` (with a comment saying they must match). A student
flat wants "beer" and "cleaning", a travel group wants "fuel" and "tickets".

- Categories per group: the twelve as defaults for every group, plus custom
  ones with a name and a colour. The CHECK constraint goes; expenses reference a
  category row.
- The web app reads the group's categories from the group detail endpoint
  instead of the hard-coded list, which removes the duplicate.
- The same duplication exists for currencies (`domain/currency.rs` and
  `web/src/lib/money.ts`); serve that list from the API too, as `GET
  /api/currencies`, in the same pull request or the next.
- Tricky: renaming or deleting a category with expenses. Rename is fine;
  delete moves its expenses to "general" after a confirmation.
- Tests: database tests for custom categories, deleting with expenses, and
  validation of a category from another group.

### 20. Spending summaries

The app answers "who owes whom" well and "where did the money go" not at all.
A flat wants to see what utilities cost this winter; a trip wants the total per
person.

- `GET /api/groups/{id}/summary?from=&to=` with totals by category, by month,
  and per member (what each paid and what each consumed, which are different
  numbers).
- A summary page in the web app with a simple bar chart per month and a table
  by category, drawn with SVG in a component, no chart library needed.
- The sums in SQL with `GROUP BY` over the existing indexes, or in
  `domain/` over the rows already loaded for balances; for summaries SQL is
  the right tool.
- Tricky: "consumed" is each member's share (`expense_shares.amount_cents`),
  "paid" is the payer's amount; with item 11 and item 13 both change.
- Tests: database tests for the totals on a seeded group, date ranges and empty
  groups.

### 21. All my balances in one place

The groups list shows my balance per group. Someone in five groups wants one
answer: in total, do I owe or am I owed, and to whom across groups.

- A summary at the top of the groups page: total owed and owing per currency
  (groups can have different currencies, so no single total), and the people I
  owe across groups.
- People are matched across groups by user account; placeholders are per group
  and stay separate.
- The data is in `GET /api/groups` already (`my_balance` per group); the per
  person view needs a new endpoint that runs the balance calculation per group.
- Tricky: suggestions are per group. "You owe Maya 20 in the flat and she owes
  you 15 from the trip" is not a single transfer unless both groups agree; show
  it as information, not as a settlement.
- Tests: database tests for a user in several groups and currencies.

### 22. How to pay me

Settle up tells me to pay Jonas 42 euros, and then I have to ask Jonas for his
account details. Every settle up ends with that message.

- Payment details on the profile: free text (IBAN, a PayPal.me or Revolut
  handle, "cash"), shown to the people who owe me on the balances page next to
  the suggestion.
- Optional "copy IBAN" and a link for handles that have a public URL format;
  no payment service is called.
- An EPC QR code (the European standard for SEPA credit transfers that banking
  apps scan) for euro groups, generated in the browser from IBAN, name and
  amount.
- Tricky: privacy. Details are visible only to members of groups I share, never
  through invite previews.
- Tests: database tests for who sees the details; a Vitest test for building the
  EPC QR payload (`web/src/lib`).

### 23. Email notifications and reminders

Nobody knows a new expense was added until they open the app. And a balance can
sit open for months because nobody likes to ask.

- Opt-in emails per user: "added to an expense" (batched, at most one mail an
  hour), a weekly summary per group, and a "reminder" button on the balances
  page that sends one polite mail to someone who owes, at most once a week per
  pair.
- Mail sending from item 7; a small queue table that the API's background task
  (the same as item 14) works through, so a slow SMTP server does not slow
  requests.
- An unsubscribe link in every mail that works without signing in (signed
  token).
- Tricky: placeholders have no email. Reminders are only for members with
  accounts.
- Depends on item 7.
- Tests: database tests with a collecting mail transport: batching, opt-out,
  the reminder limit.

### 24. Closing a group

A trip ends, everyone settles up, and the group stays in the list forever with
"settled". There is no way to archive it, and anyone can still add an expense
by mistake.

- "Close group" for the owner when all balances are zero: the group becomes
  read-only (no new or edited expenses or settlements) and moves to an
  "Archived" section of the groups list. "Reopen" undoes it.
- Every write handler checks the group is open, through one helper next to
  `access::membership`.
- Tricky: closing with open balances. Refuse, with the balances shown, rather
  than letting a group with debts disappear from view.
- Tests: database tests that each write endpoint refuses on a closed group, and
  that reopening works.

### 25. Who may change what

Every member can edit or delete every expense and settlement in the group. In a
couple that is fine; in a group of twelve on a ski trip, one careless edit
changes everyone's balance, and the history only shows it afterwards.

- A group setting: "anyone can edit anything" (today) or "only the person who
  added it and the owner". `update_expense`, `delete_expense` and the
  settlement delete check `created_by` against the signed-in user.
- An `admin` role between owner and member, for groups that want more than one
  person to manage.
- Tricky: placeholders. Expenses entered on behalf of a placeholder were added
  by a real user, so `created_by` still decides.
- Tricky: the CHECK constraint on `members.role` and `Membership::is_owner` both
  know two roles; adding a third means a migration and a `require_admin`.
- Tests: database tests for each setting and role, for edit and delete on
  expenses and settlements.

### 26. Moving groups in and out

The CSV export (`export.rs`) is for spreadsheets: it cannot be imported again,
and there is no import at all. People arriving from Splitwise have years of
history there, and Splitwise's per-group CSV export is the usual way out.

- Import a Splitwise CSV into a new group: one placeholder per person column,
  each row an expense with exact shares worked out from the per-person columns,
  payments as settlements. A preview step shows what will be created and the
  resulting balances, which must match the last line of the file.
- A full JSON export of a group (members, expenses with shares and history,
  settlements, comments) and an import of that file, for moving between
  instances.
- Tricky: Splitwise's CSV gives each person's net effect per row, not who paid.
  The payer is the one with the positive cell; rows with two positive cells are
  several payers (item 11) or have to be split into two expenses.
- Tricky: currencies. A Splitwise group can mix currencies; refuse the file
  unless item 15 is in, or import one currency per group.
- Tests: unit tests for the CSV reader with real-looking fixture files; database
  tests for import and the balance check.

### 27. Tests for the web pages

Vitest covers the helpers in `web/src/lib`, but no Svelte page or component has
a test: not the expense form, which holds the most logic on the page side, not
the join flow with its placeholder choice, not the error display. svelte-check
catches types, not behaviour.

- `@testing-library/svelte` with `jsdom` in Vitest for components:
  `ExpenseForm.svelte` (each split method, validation messages from the API
  error shape, editing an existing expense), `SettlementForm.svelte`, and the
  join page.
- A fake `fetch` through `web/src/lib/api.ts`, which already wraps requests in
  one place.
- Optionally one Playwright smoke test against the compose stack, run in CI
  after `docker compose build`, that signs in as the demo user and adds an
  expense.
- Tricky: Svelte 5 runes in tests need the Svelte plugin in Vitest's config
  (`vite.config.ts`), and `session.svelte.ts` reads `localStorage` at import.
- Tests: the item is the tests.

### 28. Accessibility pass

Things to check page by page:

- The expense form's split table: each amount input needs a label that names
  the member, and the running "left to assign" total should be announced.
- Money is shown with colour for owing and owed (`BalanceBadge.svelte`); the
  text says it too, but contrast in both themes needs checking.
- Avatars (`Avatar.svelte`) with initials need to be hidden from screen readers
  where the name is next to them.
- After a save or delete, focus should go somewhere sensible instead of the top
  of the page, and errors should be announced.
- Approach: axe and a screen reader per page, fixes, and component tests from
  item 27 that assert the labels and roles, so they do not regress.
- Depends on item 27 for the tests.
- Tests: component tests for labels and announcements.

### 29. Running a private instance

Two things for people who host this for friends. Anyone who reaches the app can
sign up. And `docker-compose.yml` publishes the API on port 4407 next to nginx,
so the API can be reached directly, without the headers from item 3 and with a
client-chosen `X-Forwarded-For` if item 2 trusts it.

- `SIGNUP=open|invite|closed`: `invite` allows signup only from a join link,
  `closed` only through a console command (`api create-user`, next to the
  existing `api healthcheck` subcommand).
- Publish the API port only on `127.0.0.1` by default (`127.0.0.1:4407:4407`),
  or not at all; the web container reaches it on the compose network.
- `docs/operations.md`: putting the stack behind Caddy or Traefik with HTTPS,
  and which forwarded headers to trust.
- Depends on item 2 for the trusted proxy logic.
- Tests: router tests for each signup mode; the compose change is checked by
  hand.

## Maybe

### 30. Large groups

Balances are never stored: `compute_for_group` loads every share row of every
expense of the group on each request to the balances page, and the groups list
runs four correlated sums per group for "my balance". That is the right choice
for correctness and fine for a flat with a few hundred expenses a year. A
long-lived group with ten thousand expenses, or a user with many groups, would
feel it.

- Measure first: a database test fixture with a group of 20,000 expenses and
  `EXPLAIN (ANALYZE, FORMAT JSON)` on the balance queries, asserting on rows
  read, not time.
- Sum in SQL: per member, `sum(paid)` and `sum(share)` with `GROUP BY` instead
  of loading rows into Rust; `balance::compute` keeps the logic for tests and
  the export.
- One query for the groups list balances with `GROUP BY` instead of four
  subqueries per group.
- `simplify` is exact up to 16 people with open balances and greedy beyond
  (`EXACT_LIMIT`). That is fine; a test that the greedy result is never worse
  than one transfer per debtor documents it.
- Undecided: storing running balances per member. Faster, but a second source
  of truth that can drift, which the README says is avoided on purpose. I would
  only do the SQL sums.
- Tests: the fixture and query plan checks above.

### 31. Adding expenses offline

Expenses happen where the signal is bad: in the mountains, abroad without data.
The web app needs the API for everything.

- A service worker that caches the app shell and the groups the user opened,
  and a queue in IndexedDB for new expenses made offline, sent when back online
  with the idempotency keys from item 5 so a retry never creates a duplicate.
- Offline expenses show as "not sent yet" in the list.
- Tricky: conflicts. An expense queued offline may refer to a member who left
  meanwhile; the API's 422 must come back to the user as a fixable entry, not a
  silent drop.
- Tricky: the static adapter and nginx's `Cache-Control: no-cache` on `/`
  interact with the service worker's cache; versioning must be right, or users
  run an old app against a new API.
- Depends on item 5.
- Tests: Vitest tests for the queue module; the service worker by hand.

### 32. Translations and local formats

The interface is English, while amounts already accept `12,50` and `1 234.56`
(`parseAmount` in `web/src/lib/money.ts`). Travel groups are often mixed in
language.

- A small i18n setup in the web app (message catalogues per language, plural
  rules through `Intl.PluralRules`), the language from the browser with a
  choice in settings.
- Money and dates through `Intl.NumberFormat` and `Intl.DateTimeFormat` with the
  user's locale, keeping the currency's minor units from the API.
- API error messages are English text in `AppError`; the web app should map
  error codes and field codes to its own messages rather than translating the
  API.
- Tricky: activity texts are built in `web/src/lib/activity.ts` from kinds and
  payloads, which suits translation; history diffs in `history.ts` too.
- Undecided: whether a second language will be maintained. I would only start
  with someone who uses it.
- Tests: Vitest tests for formatting in two locales and for message keys that
  exist in every catalogue.

### 33. Two-step sign in

The account holds a record of who paid what, sometimes with payment details
(item 22). A password is the only protection.

- TOTP with the `totp-rs` crate, set up from the settings page with a QR code,
  plus single-use recovery codes stored hashed. Login returns "code needed" and
  a short-lived challenge token; a second call with the code returns the
  session token.
- Depends on item 1: a second factor is pointless while tokens cannot be
  revoked.
- Tricky: password reset (item 7) must not skip the second step.
- Tests: database tests for enabling, signing in with a code, a wrong code, and
  a recovery code used once.

### 34. Metrics for monitoring

There is `/healthz` and structured logs with `LOG_FORMAT=json`. Someone running
this for a few groups might want request counts, error rates and latency in
Prometheus or similar, without reading logs.

- `GET /metrics` in Prometheus text format with `metrics` and
  `metrics-exporter-prometheus`: request count and latency by route template
  and status, database pool usage, and counts of users, groups and expenses.
- Not proxied by nginx, so it stays on the internal network unless the operator
  publishes it.
- Undecided: whether anyone would scrape it. It is small, which is why it is
  here at all.
- Tests: a router test that the endpoint answers and contains the route label
  of a request made just before.

### 35. Personal API tokens and a described API

The API is JSON and documented as a table in the README. Scripts that add an
expense from a phone shortcut, or a bank import (item 36), need a token that is
not a week-long login copied out of `localStorage`, and a description of the
request bodies.

- Personal tokens: named, hashed at rest, shown once, revocable, optionally read
  only, accepted by `AuthUser` next to session tokens (item 1).
- An OpenAPI document generated from the handlers with `utoipa`, served at
  `/api/openapi.json`, and the README's table linked to it.
- Depends on item 1 for the session table the tokens sit next to.
- Tricky: `utoipa` annotations on every handler and input type are a lot of
  churn; doing it route module by route module keeps pull requests small.
- Tests: database tests for a token's access and revocation; a test that the
  OpenAPI document lists every route in `router()`.

### 36. Importing a bank statement

Most shared expenses already exist as lines on someone's bank statement. Typing
them again is the main chore of using an app like this.

- Upload a CSV statement (or CAMT.053 XML, which most European banks offer),
  map its columns once per bank, pick the lines that were shared, and turn each
  into an expense paid by the uploader with the group's default split (item 18
  helps here).
- Remember imported lines by a hash of date, amount and text, so importing an
  overlapping statement does not create duplicates.
- Tricky: every bank's CSV is different (separators, decimal commas, date
  formats, signs). The column mapping step is most of the work, and the parsing
  should live in a `domain/` module with many fixture files.
- Tricky: statements contain other private payments. Nothing is stored except
  the lines the user picks.
- Depends on item 26 for the import preview flow it shares.
- Undecided: whether this is worth it over typing. For a household with many
  shared bills it is; for a weekend trip it is not. Hence Maybe.
- Tests: unit tests for the readers and the duplicate hash; database tests for
  importing picked lines.
