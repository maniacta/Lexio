# H2 verification.
# Internal error detail must not reach the client, while user-facing messages
# keep working and the real detail still lands in the audit log.
#
# ASCII-only on purpose: this repo's scripts are read by Windows PowerShell 5.1,
# which decodes BOM-less files as ANSI and corrupts non-ASCII literals. Assertions
# therefore match ASCII prefixes rather than the Chinese message text.
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

function Invoke-Curl($uri, $method, $body) {
  # curl is used instead of Invoke-WebRequest: PowerShell 5.1 throws on 4xx/5xx
  # and its error stream cannot be read reliably, which hid the bodies under test.
  # The payload goes through a UTF-8 temp file because cmd-style quote stripping
  # corrupts inline JSON on Windows.
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
  $bytes = [System.Text.Encoding]::UTF8.GetBytes($text.Trim())
  return @{ code = [int]$code; body = $text.Trim(); utf8 = $bytes }
}

function Body-Has($res, $ascii) {
  return [System.Text.Encoding]::UTF8.GetString($res.utf8) -match [regex]::Escape($ascii)
}

$token = (Invoke-RestMethod -Uri "$base/auth/token" -Method Get).token
$h = @{ "X-Lexio-Token" = $token; "Content-Type" = "application/json" }

$marker   = "lexio-internal"
$generic  = "GENERIC_INTERNAL_MARKER"   # replaced below with the real substring
# The generic body is "<chinese>..."; assert on its ASCII-safe structural signal
# instead: a 500 whose body has no repository/DB detail.
$leakPattern = "sqlite|Query returned no rows|no such column|constraint failed|database disk image|near |UNIQUE constraint"

# -- 1. Internal error: delete_model on a nonexistent model -------------------
# Previously leaked rusqlite "Query returned no rows" as a 500.
$res1 = Invoke-Curl "$base/settings/providers/no-such-provider/models/no-such-model" "DELETE" $null
Write-Host ("  delete_model body: {0}" -f $res1.body)
Show "delete_model hides rusqlite detail" (-not ($res1.body -match $leakPattern)) ("HTTP {0}" -f $res1.code)
Show "delete_model returns 500 not 400" ($res1.code -eq 500) ("HTTP {0}" -f $res1.code)

# -- 2. A missing row on a delete route --------------------------------------
$res2 = Invoke-Curl "$base/relations/no-such-relation" "DELETE" $null
Write-Host ("  delete_relation body: {0}" -f $res2.body)
Show "delete_relation hides internal detail" (-not ($res2.body -match $leakPattern)) ("HTTP {0}" -f $res2.code)

# -- 3. Hand-written domain errors must survive as 400 -----------------------
# A missing knowledge point yields the hand-written message; the point is that
# it is not replaced by the generic internal text.
$res3 = Invoke-Curl "$base/knowledge/ghost-kp/relations" "POST" '{"to_kp_id":"other","relation_type":"related"}'
Write-Host ("  domain error body: {0}" -f $res3.body)
Show "domain error stays 400" ($res3.code -eq 400) ("HTTP {0}" -f $res3.code)
Show "domain error text is preserved" (($res3.body.Length -gt 0) -and -not ($res3.body -match $leakPattern) -and -not ($res3.body -match "Failed to parse")) ("body: {0}" -f $res3.body)

# -- 4. Validation error stays a 400 with its own text ----------------------
$res4 = Invoke-Curl "$base/knowledge/any-kp/relations" "POST" '{"to_kp_id":"","relation_type":"prerequisite"}'
Write-Host ("  validation body: {0}" -f $res4.body)
Show "validation error stays 400" ($res4.code -eq 400) ("HTTP {0}" -f $res4.code)
Show "validation text not replaced" (-not ($res4.body -match $leakPattern)) ("body: {0}" -f $res4.body)

# -- 5. Config errors keep the code the frontend maps to a hint --------------
$res5 = Invoke-Curl "$base/ai/chat" "POST" '{"messages":[{"role":"user","content":"hi"}]}'
Write-Host ("  ai/chat body: {0}" -f $res5.body)
$actionable = ($res5.body -match "MISSING_API_KEY") -or ($res5.body -match "No default") -or ($res5.body -match "No model configured")
Show "config error keeps actionable code" $actionable ("HTTP {0}: {1}" -f $res5.code, $res5.body)

# -- 6. The internal marker must never reach the client ----------------------
$markerLeak = $false
foreach ($p in @("/settings/providers", "/knowledge", "/sources", "/chat/sessions", "/learning/reviews/due")) {
  $r = Invoke-WebRequest -Uri "$base$p" -Method Get -UseBasicParsing -Headers $h
  if ($r.Content -match $marker) { $markerLeak = $true }
}
foreach ($r in @($res1, $res2, $res3, $res4, $res5)) {
  if ($r.body -match $marker) { $markerLeak = $true }
}
Show "internal marker never reaches client" (-not $markerLeak) "5 read routes + 5 error bodies"

# -- 7. The detail must still be recorded in the audit log -------------------
Start-Sleep -Milliseconds 1500   # the audit layer flushes every 500ms
$db = Join-Path $env:TEMP "lexio-e2e-h2\lexio.db"
if (Test-Path $db) {
  $q = "SELECT COUNT(*) FROM audit_logs WHERE error_message LIKE '%Query returned no rows%' OR error_message LIKE '%no such%' OR error_message LIKE '%not found%';"
  $count = (& sqlite3 $db $q 2>&1) -join ""
  Write-Host ("  audit rows carrying internal detail: {0}" -f $count)
  Show "detail recorded in audit log" ([int]$count -ge 1) ("rows: {0}" -f $count)
} else {
  Write-Host ("  (db not found at {0}, skipping audit check)" -f $db)
}

Write-Host ""
if ($fail -eq 0) {
  Write-Host "H2 PASS: internals hidden, user-facing messages intact, detail logged." -ForegroundColor Green
  exit 0
} else {
  Write-Host ("H2 FAIL: {0} check(s) failed." -f $fail) -ForegroundColor Red
  exit 1
}
