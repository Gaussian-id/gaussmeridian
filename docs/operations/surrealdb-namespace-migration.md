# SurrealDB namespace migration — `gaussrouter` → `gaussmeridian`

**Status:** required once per environment that holds data written before the
`refactor/gaussmeridian-rename` change.

## Why this exists

The rename changed the SurrealDB namespace **value**, not just the variable name:

```diff
- GAUSSROUTER_DB_NAMESPACE=gaussrouter
+ GAUSSMERIDIAN_DB_NAMESPACE=gaussmeridian
```

SurrealDB has no `ALTER NAMESPACE ... RENAME`. A namespace is part of every record's
address, so pointing the app at a new namespace makes the old data **invisible** — it is
not deleted, it is simply no longer addressed. Any environment with real data must be
exported from the old namespace and imported into the new one before first boot on the
renamed code.

## Does this apply to me?

```bash
# Any rows under the old namespace?
curl -s -X POST http://127.0.0.1:8001/sql \
  -u "root:$SURREALDB_PASSWORD" \
  -H 'surreal-ns: gaussrouter' -H 'surreal-db: main' \
  -H 'Accept: application/json' \
  --data-raw 'INFO FOR DB;'
```

- Returns tables → **migrate** (below).
- Returns an empty/no-such-namespace result → nothing to do; boot straight onto the new name.

> The dev volume `0code_surrealdb-data` currently cannot be opened at all (SurrealDB 2.0.0
> hangs at "Starting kvs store"), so this migration cannot run against it until that volume
> is repaired or discarded. See `docs/operations/service-startup-runbook.md`.

## Migration

Run with the stack **up** and the API **stopped** (no writers), against SurrealDB directly.

```bash
# 1. Bring up only the datastore
docker compose up -d surrealdb
docker compose stop gaussmeridian

# 2. Export the old namespace
docker compose exec surrealdb /surreal export \
  --endpoint http://localhost:8000 \
  --user root --pass "$SURREALDB_PASSWORD" \
  --namespace gaussrouter --database main \
  /data/gaussrouter-main.surql

# 3. Import into the new namespace (same database name)
docker compose exec surrealdb /surreal import \
  --endpoint http://localhost:8000 \
  --user root --pass "$SURREALDB_PASSWORD" \
  --namespace gaussmeridian --database main \
  /data/gaussrouter-main.surql

# 4. Verify row parity on the tables that matter
for T in users project api_key ledger route_decision; do
  for NS in gaussrouter gaussmeridian; do
    printf '%-14s %-14s ' "$NS" "$T"
    curl -s -X POST http://127.0.0.1:8001/sql \
      -u "root:$SURREALDB_PASSWORD" \
      -H "surreal-ns: $NS" -H 'surreal-db: main' \
      --data-raw "SELECT count() FROM $T GROUP ALL;"
    echo
  done
done
```

Counts must match per table before you continue.

```bash
# 5. Boot the app on the new namespace
docker compose up -d gaussmeridian
docker compose logs -f gaussmeridian | head -40
```

## Repeat per database

The export/import is **per (namespace, database) pair**. If an environment uses more than
`main` — e.g. the disposable `prd21_dev` from `docker-compose.override.yml` — repeat steps
2–4 for each `--database` value.

## Rollback

Nothing is destroyed: the old `gaussrouter` namespace is left intact by this procedure.
To roll back, set `GAUSSMERIDIAN_DB_NAMESPACE=gaussrouter` and restart. Only drop the old
namespace once you are satisfied:

```sql
-- destructive, and only after verified parity + a backup
REMOVE NAMESPACE gaussrouter;
```

## Related

- `docs/operations/service-startup-runbook.md`
- `docker-compose.override.yml` (pins the disposable `prd21_dev` database)
