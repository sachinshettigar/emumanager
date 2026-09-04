# Playbook: change the database schema or a query

`sqlx` checks queries at compile time against `.sqlx/` offline metadata, which is committed so CI
and agents don't need a live DB. Any query change needs that metadata refreshed.

## New migration

```
just db-migrate "add_tags_to_emulators"      # creates migrations/<ts>_add_tags_to_emulators.sql
```

- Write forward-only SQL. SQLite: prefer additive changes; for a column drop/rename use the
  12-step table rebuild or a new table + copy.
- Migrations run automatically in `Registry::open` via `sqlx::migrate!()`.
- Update `docs/context/domain-model.md` if the change is user-visible or structural.

## After changing any `query!` / `query_as!`

```
just db-prepare        # regenerates .sqlx/  (wraps: cargo sqlx prepare --workspace)
git add .sqlx migrations
```

Verify offline build works with no `DATABASE_URL`:

```
SQLX_OFFLINE=true cargo build --workspace
cargo sqlx prepare --check --workspace    # part of `just validate`
```

## Tests

- Open a `Registry` in a `tempdir`, run migrations, exercise the new column/query.
- If a migration transforms data, test the transform with a fixture row.

## Gotcha

`lefthook` pre-commit runs `just db-prepare` and stages `.sqlx/`. If `.sqlx/` and your queries
disagree in CI, you committed a query change without re-preparing (or with `--no-verify`).
