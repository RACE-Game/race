param(
    [string]$DbName = "race_poker_product",
    [string]$User = "postgres",
    [string]$AdminDb = "postgres"
)

$ErrorActionPreference = "Stop"

$schemaPath = Join-Path $PSScriptRoot "..\\sql\\product_schema_v1.sql"
$schemaPath = [System.IO.Path]::GetFullPath($schemaPath)

$dbExists = psql -U $User -d $AdminDb -Atc "SELECT 1 FROM pg_database WHERE datname = '$DbName';"
if (-not $dbExists) {
    psql -U $User -d $AdminDb -c "CREATE DATABASE $DbName;"
}

psql -U $User -d $DbName -f $schemaPath

Write-Host "Initialized Postgres product DB '$DbName' using schema $schemaPath"
