# A server to simulate a blockchain for development and test

## Current local storage behavior

`race-facade` now uses:

- sqlite for gameplay/runtime-facing facade state
- Postgres for product-layer guest data when available

Default local dev behavior:

- sqlite db path: `data/facade.sqlite3`
- default Postgres product db url: `postgresql://postgres@localhost/race_poker_product`

Startup priority:

1. `--product-db-url`
2. `RACE_FACADE_PRODUCT_DB_URL`
3. default local Postgres dev db
4. sqlite-only fallback if the default local Postgres db is unavailable

To force sqlite-only mode even when local Postgres exists:

```powershell
$env:RACE_FACADE_DISABLE_DEFAULT_PRODUCT_DB='1'
cargo run -p race-facade
```

To initialize the local Postgres product db:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\init_product_db.ps1
```
