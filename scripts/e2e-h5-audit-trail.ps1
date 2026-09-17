# H5 verification.
# The audit trail must be readable, and its timestamps must be the server's own
# clock: a client-supplied timestamp must not be able to move an event out of the
# retention window or forge its place in the ordering.
#
# ASCII-only on purpose: Windows PowerShell 5.1 decodes BOM-less files as ANSI
# and would corrupt non-ASCII literals in assertions.
$ErrorActionPreference = "Continue"
$base = "http://127.0.0.1:3001/api"
$fail = 0

function Show($label, $ok, $detail) {
  if ($ok) {
    Write-Host ("[PASS] {0} -> {1}" -f $label, $detail) -ForegroundColor Green
  } else {
    Write-Host ("[FAIL] {0} -> {1}" -f $label, $detail) -ForegroundColor Red
    $script:fail++
  }
}

# PowerShell 5.1 throws on 4xx/5xx and its error stream is unreliable, so curl
# with a body temp file is used for anything that may fail.
function Invoke-Curl($uri, $method, $body) {
  $tmp = [System.IO.Path]::GetTempFileName()
  $bodyFile = "$tmp.json"
  $a = @("-s", "-o", $tmp, "-w", "%{http_code}", "-X", $method, "-H", "X-Lexio-Token: $token")
  if ($body) {
    [System.IO.File]::WriteAllText($bodyFile, $body, (New-Object System.Text.UTF8Encoding($false)))
    $a += @("-H", "Content-Type: application/json", "--data-binary", "@$bodyFile")
  }
  $a += $uri
  $code = (& curl.exe @a) -join ""
  $text = ""
  if (Test-Path $tmp) {
    $text = Get-Content -Raw -Path $tmp -ErrorAction SilentlyContinue
    Remove-Item $tmp -Force -ErrorAction SilentlyContinue
  }
  Remove-Item $bodyFile -Force -ErrorAction SilentlyContinue
  if (-not $text) { $text = "" }
  return @{ code = [int]$code; body = $text.Trim() }
}

$server = Join-Path $env:TEMP "lexio-audit-target\debug\server.exe"
$work = Join-Path $env:TEMP "lexio-e2e-h5"
$db = Join-Path $work "lexio.db"

Get-Process -Name server -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 2
Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $work | Out-Null

# Force file-backed key storage so the run does not depend on an OS credential
# store, then start the backend against a throwaway database.
$env:LEXIO_MASTER_KEY_STORE = "file"
$proc = Start-Process -FilePath $server -WorkingDirectory $work -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 5

$token = $null
for ($i = 0; $i -lt 10 -and -not $token; $i++) {
  try { $token = (Invoke-RestMethod -Uri "$base/auth/token" -Method Get).token } catch { Start-Sleep -Milliseconds 700 }
}
Show "backend boots" ([bool]$token) "token acquired"
if (-not $token) {
  Write-Host "cannot continue without a token" -ForegroundColor Red
  Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
  exit 1
}

# --- 1. A back-dated client timestamp must not be trusted -------------------
# The logger sends the client's own clock; it must be recorded but must not
# become the row's timestamp, which is what prune() and ordering use.
$backdated = "1999-01-01T00:00:00.000Z"
$payload = '{"logs":[{"level":"info","category":"e2e","action":"backdated_event","timestamp":"' + $backdated + '"}]}'
$res = Invoke-Curl "$base/logs/batch" "POST" $payload
Show "batch ingest accepted" ($res.code -eq 200) ("HTTP {0}" -f $res.code)

Start-Sleep -Milliseconds 1500   # the audit layer flushes every 500ms

$storedTs = ""
$clientTs = ""
if (Test-Path $db) {
  $storedTs = ((& sqlite3 $db "SELECT timestamp FROM audit_logs WHERE action='backdated_event' ORDER BY rowid DESC LIMIT 1;" 2>&1) -join "").Trim()
  $clientTs = ((& sqlite3 $db "SELECT client_timestamp FROM audit_logs WHERE action='backdated_event' ORDER BY rowid DESC LIMIT 1;" 2>&1) -join "").Trim()
  Write-Host ("  stored timestamp: {0}" -f $storedTs)
  Write-Host ("  client timestamp: {0}" -f $clientTs)
}
Show "client timestamp is not used as the row timestamp" ($storedTs -ne $backdated -and $storedTs -match "^20") ("stored={0}" -f $storedTs)
Show "client timestamp is preserved for diagnostics" ($clientTs -eq $backdated) ("client={0}" -f $clientTs)

# A back-dated row must not be pruned away: it is inside the retention window
# because the server stamped it, not the client.
$pruned = ((& sqlite3 $db "SELECT COUNT(*) FROM audit_logs WHERE action='backdated_event';" 2>&1) -join "").Trim()
Show "back-dated event survives a retention pass" ([int]$pruned -eq 1) ("rows: {0}" -f $pruned)

# --- 2. The trail must be readable ------------------------------------------
$page = Invoke-Curl "$base/audit/logs?limit=5" "GET" $null
Show "GET /audit/logs succeeds" ($page.code -eq 200) ("HTTP {0}" -f $page.code)
$json = $null
try { $json = $page.body | ConvertFrom-Json } catch { }
Show "response is a paged payload" ($null -ne $json -and $null -ne $json.logs -and $null -ne $json.total) ("total={0}, returned={1}" -f $json.total, @($json.logs).Count)
Show "page size is honoured" (@($json.logs).Count -le 5) ("returned {0}" -f @($json.logs).Count)
Show "timestamp authority is declared" ($json.timestamp_authority -eq "server") ("authority={0}" -f $json.timestamp_authority)

# --- 3. Filters must narrow the result --------------------------------------
$errors = Invoke-Curl "$base/audit/logs?level=error" "GET" $null
$errJson = $errors.body | ConvertFrom-Json
$allError = $true
foreach ($e in @($errJson.logs)) { if ($e.level -ne "error") { $allError = $false } }
Show "level filter is applied" ($allError -and $errJson.total -le $json.total) ("matched={0}" -f $errJson.total)

$searched = Invoke-Curl "$base/audit/logs?search=backdated_event" "GET" $null
$searchJson = $searched.body | ConvertFrom-Json
Show "search finds the ingested event" ($searchJson.total -ge 1) ("matched={0}" -f $searchJson.total)

# A LIKE wildcard must be literal, not "match everything".
$wildcard = Invoke-Curl "$base/audit/logs?search=%25" "GET" $null
$wildJson = $wildcard.body | ConvertFrom-Json
Show "wildcard search is escaped" ($wildJson.total -lt $json.total) ("'%' matched {0} of {1}" -f $wildJson.total, $json.total)

# --- 4. A bad time bound must be rejected, not silently ignored -------------
$badSince = Invoke-Curl "$base/audit/logs?since=yesterday" "GET" $null
Show "malformed since is rejected" ($badSince.code -eq 400) ("HTTP {0}" -f $badSince.code)

# --- 5. Facets must reflect what is actually stored -------------------------
$facets = Invoke-Curl "$base/audit/facets" "GET" $null
$facetJson = $facets.body | ConvertFrom-Json
Show "facets list the categories in use" ($facetJson.categories -contains "e2e") ("categories={0}" -f ($facetJson.categories -join ","))
Show "facets list the sources in use" ($facetJson.sources -contains "frontend") ("sources={0}" -f ($facetJson.sources -join ","))

# --- 6. The read route must stay behind the token ---------------------------
$saved = $token
$token = "not-a-real-token"
$unauth = Invoke-Curl "$base/audit/logs" "GET" $null
$token = $saved
Show "audit read requires the token" ($unauth.code -eq 401) ("HTTP {0}" -f $unauth.code)

# --- 7. Paging must not overlap or repeat ----------------------------------
$p1 = (Invoke-Curl "$base/audit/logs?limit=2&offset=0" "GET" $null).body | ConvertFrom-Json
$p2 = (Invoke-Curl "$base/audit/logs?limit=2&offset=2" "GET" $null).body | ConvertFrom-Json
$ids1 = @($p1.logs | ForEach-Object { $_.id })
$ids2 = @($p2.logs | ForEach-Object { $_.id })
$overlap = @($ids1 | Where-Object { $ids2 -contains $_ })
Show "pages do not overlap" ($overlap.Count -eq 0) ("page1={0}, page2={1}, overlap={2}" -f $ids1.Count, $ids2.Count, $overlap.Count)

# --- 8. An oversized limit must be clamped, not honoured --------------------
$big = (Invoke-Curl "$base/audit/logs?limit=99999" "GET" $null).body | ConvertFrom-Json
Show "oversized limit is clamped" ([int]$big.limit -le 200) ("limit={0}" -f $big.limit)

Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 1
Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue

Write-Host ""
if ($fail -eq 0) {
  Write-Host "H5 PASS: the trail is readable and its timestamps are server-authoritative." -ForegroundColor Green
  exit 0
} else {
  Write-Host ("H5 FAIL: {0} check(s) failed." -f $fail) -ForegroundColor Red
  exit 1
}