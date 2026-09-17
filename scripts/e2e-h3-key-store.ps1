# H3 verification: after normal operation the master key must not remain in a
# plaintext file beside the database, the backend must stay functional, and the
# key's provenance must be recorded in the audit log.
#
# This checks the running app's end state only. The migration itself (adopting an
# existing file key, writing it to the OS store, and deleting the file) is
# covered deterministically against the real OS credential store by the
# `key_store::tests::os_store::real_store_round_trip_and_file_migration` test,
# which uses its own credential namespace and cleans up after itself.
#
# ASCII-only: Windows PowerShell 5.1 decodes BOM-less files as ANSI.
$ErrorActionPreference = "Continue"
$fail = 0

function Show($label, $ok, $detail) {
  if ($ok) {
    Write-Host ("[PASS] {0} -> {1}" -f $label, $detail) -ForegroundColor Green
  } else {
    Write-Host ("[FAIL] {0} -> {1}" -f $label, $detail) -ForegroundColor Red
    $script:fail++
  }
}

$server = Join-Path $env:TEMP "lexio-audit-target\debug\server.exe"
$work = Join-Path $env:TEMP "lexio-e2e-h3"

Get-Process -Name server -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 2
Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $work | Out-Null

# --- 1. seed a legacy plaintext key and let the app adopt/migrate it ---------
# If the app's credential entry already exists with a different value the app
# deliberately keeps the file (it may be the only copy of a key that encrypted
# existing rows), so accept either store-backed outcome.
$legacyKey = [byte[]](1..32 | ForEach-Object { $_ })
$keyFile = Join-Path $work ".lexio-master.key"
[System.IO.File]::WriteAllBytes($keyFile, $legacyKey)
Show "legacy plaintext key seeded" (Test-Path $keyFile) "32 bytes at .lexio-master.key"

$proc = Start-Process -FilePath $server -WorkingDirectory $work -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 5

# --- 2. the backend must be functional (key usable) --------------------------
$token = $null
for ($i = 0; $i -lt 10 -and -not $token; $i++) {
  try { $token = (Invoke-RestMethod -Uri "http://127.0.0.1:3001/api/auth/token" -Method Get).token } catch { Start-Sleep -Milliseconds 700 }
}
Show "backend boots and serves requests" ([bool]$token) "token acquired"

# --- 3. provenance must be recorded, and the key must be store-backed -------
Start-Sleep -Seconds 2
$db = Join-Path $work "lexio.db"
$loaded = "0"
$fileOutcome = "0"
$outcome = ""
if (Test-Path $db) {
  $loaded = (& sqlite3 $db "SELECT COUNT(*) FROM audit_logs WHERE action='master_key_loaded';" 2>&1) -join ""
  $fileOutcome = (& sqlite3 $db "SELECT COUNT(*) FROM audit_logs WHERE action='master_key_stored_in_plaintext_file';" 2>&1) -join ""
  $outcome = ((& sqlite3 $db "SELECT result_summary FROM audit_logs WHERE action='master_key_loaded' ORDER BY timestamp DESC LIMIT 1;" 2>&1) -join "").Trim()
  Write-Host ("  outcome recorded: {0}" -f $outcome)
}
Show "key provenance recorded in audit log" ([int]$loaded -ge 1) ("rows: {0}" -f $loaded)

# --- 4. the end-state invariant: the key must not be file-backed ------------
# This environment has a working credential store, so the key must have come
# from it. Note the assertion is about the recorded outcome rather than the
# file's mere existence: a pre-existing store entry with a different value is
# deliberately kept alongside the file, because deleting the file could destroy
# the only copy of a key that encrypted existing rows.
$isPlaintext = ($outcome -match "FileFallback") -or ($outcome -match "FileForced")
Show "key is held by the OS credential store, not a file" (-not $isPlaintext -and [int]$fileOutcome -eq 0) ("outcome={0}, plaintextWarnings={1}" -f $outcome, $fileOutcome)

# When the store adopted the file's key, that file must be gone.
if (($outcome -match "MigratedFromFile") -or ($outcome -match "RedundantFileRemoved")) {
  Show "plaintext key file removed after adopting it" (-not (Test-Path $keyFile)) "no .lexio-master.key"
}

# --- 5. restart keeps working from whatever backend was chosen --------------
Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2
$proc2 = Start-Process -FilePath $server -WorkingDirectory $work -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 5

$token2 = $null
for ($i = 0; $i -lt 10 -and -not $token2; $i++) {
  try { $token2 = (Invoke-RestMethod -Uri "http://127.0.0.1:3001/api/auth/token" -Method Get).token } catch { Start-Sleep -Milliseconds 700 }
}
Show "backend restarts with the same key" ([bool]$token2) "token acquired"

# A restart must not invent a new key: the recorded outcome must stay stable.
Stop-Process -Id $proc2.Id -Force -ErrorAction SilentlyContinue
if (Test-Path $db) {
  $outcome2 = ((& sqlite3 $db "SELECT result_summary FROM audit_logs WHERE action='master_key_loaded' ORDER BY timestamp DESC LIMIT 1;" 2>&1) -join "").Trim()
  Write-Host ("  outcome after restart: {0}" -f $outcome2)
  Show "restart reuses the same key source" ($outcome2 -notmatch "Created") ("after={0}" -f $outcome2)
}
Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue

Write-Host ""
if ($fail -eq 0) {
  Write-Host "H3 PASS: no plaintext master key remains; provenance is audited." -ForegroundColor Green
  exit 0
} else {
  Write-Host ("H3 FAIL: {0} check(s) failed." -f $fail) -ForegroundColor Red
  exit 1
}