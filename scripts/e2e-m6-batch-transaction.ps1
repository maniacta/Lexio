# M6 verification.
# A batch insert must be all-or-nothing. Previously each row committed on its
# own, so a failure partway through left the earlier rows written while the
# client got a 500 and re-queued the whole batch for retry, duplicating them.
#
# A mid-batch failure cannot be reached through the endpoint's own validation --
# it rejects the whole batch with 400 before touching the DB. So the failure is
# forced with a temporary trigger on audit_logs: the first row inserts cleanly,
# the trigger aborts the second, and the question is whether the first survives.
#
# ASCII-only on purpose: Windows PowerShell 5.1 decodes BOM-less files as ANSI.
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

function Invoke-Curl($uri, $method, $bodyFile) {
  $tmp = [System.IO.Path]::GetTempFileName()
  $a = @("-s", "-o", $tmp, "-w", "%{http_code}", "-X", $method, "-H", "X-Lexio-Token: $token")
  if ($bodyFile) {
    $a += @("-H", "Content-Type: application/json", "--data-binary", "@$bodyFile")
  }
  $a += $uri
  $code = (& curl.exe @a) -join ""
  $text = ""
  if (Test-Path $tmp) {
    $text = Get-Content -Raw -Path $tmp -ErrorAction SilentlyContinue
    Remove-Item $tmp -Force -ErrorAction SilentlyContinue
  }
  if (-not $text) { $text = "" }
  return @{ code = [int]$code; body = $text.Trim() }
}

# Write text to a UTF-8 (no BOM) temp file and return its path. Used for both
# JSON payloads and SQL, because the Windows command line re-encodes arguments
# through the ANSI code page.
function Text-File($text) {
  $p = [System.IO.Path]::GetTempFileName()
  [System.IO.File]::WriteAllText($p, $text, (New-Object System.Text.UTF8Encoding($false)))
  return $p
}

function Sql($statement) {
  $file = Text-File $statement
  $out = (& sqlite3 $db ".read $file" 2>&1) -join ""
  Remove-Item $file -Force -ErrorAction SilentlyContinue
  return $out.Trim()
}

# A batch of three entries; `$middle` becomes the action of the middle one, which
# the injected trigger keys on.
function Batch($middle) {
  $entries = @()
  foreach ($action in @("before_boom", $middle, "after_boom")) {
    $entries += '{"level":"info","category":"ui","action":"' + $action + '","user_action":"click"}'
  }
  return Text-File ('{"logs":[' + ($entries -join ",") + ']}')
}

$server = Join-Path $env:TEMP "lexio-audit-target\debug\server.exe"
$work = Join-Path $env:TEMP "lexio-e2e-m6"
$db = Join-Path $work "lexio.db"

Get-Process -Name server -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 2
Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $work | Out-Null

$env:LEXIO_MASTER_KEY_STORE = "file"
$proc = Start-Process -FilePath $server -WorkingDirectory $work -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 5

$token = $null
for ($i = 0; $i -lt 10 -and -not $token; $i++) {
  try { $token = (Invoke-RestMethod -Uri "$base/auth/token" -Method Get).token } catch { Start-Sleep -Milliseconds 700 }
}
Show "backend boots" ([bool]$token) "token acquired"
if (-not $token) {
  Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
  exit 1
}

function FrontendRows {
  # Only frontend rows: the backend's own startup event shares the table.
  return [int]$((Sql "SELECT COUNT(*) FROM audit_logs WHERE source='frontend';") -replace "\D", "")
}

# --- 1. A valid batch lands whole -------------------------------------------
$before = FrontendRows
$f = Batch "second"
$res = Invoke-Curl "$base/logs/batch" "POST" $f
Remove-Item $f -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 1200   # nothing async here, but keep reads off the write
$after = FrontendRows
Write-Host ("  valid batch: HTTP {0}, frontend rows {1} -> {2}" -f $res.code, $before, $after)
Show "a valid batch is accepted" ($res.code -eq 200) ("HTTP {0}" -f $res.code)
Show "all three entries are written" (($after - $before) -eq 3) ("added: {0}" -f ($after - $before))

# --- 2. Force a failure on the middle row -----------------------------------
$err = Sql "CREATE TRIGGER m6_boom BEFORE INSERT ON audit_logs WHEN NEW.action = 'boom' BEGIN SELECT RAISE(ABORT, 'm6-injected-failure'); END;"
Write-Host ("  trigger installed: {0}" -f $err)

# Baselines for the actions this batch touches; test 1 already wrote one
# `before_boom`, so the assertions compare against these rather than zero.
$beforeBoom = [int]$((Sql "SELECT COUNT(*) FROM audit_logs WHERE action='before_boom';") -replace "\D", "")
$afterBoom = $beforeBoom

$before = FrontendRows
$f = Batch "boom"
$res = Invoke-Curl "$base/logs/batch" "POST" $f
Remove-Item $f -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 300
$after = FrontendRows
$afterBoom = [int]$((Sql "SELECT COUNT(*) FROM audit_logs WHERE action='before_boom';") -replace "\D", "")
Write-Host ("  failing batch: HTTP {0}, frontend rows {1} -> {2}" -f $res.code, $before, $after)
Show "a mid-batch failure surfaces as 500" ($res.code -eq 500) ("HTTP {0}" -f $res.code)
Show "the failed batch writes nothing" (($after - $before) -eq 0) ("added: {0}" -f ($after - $before))
Show "the row before the failure is rolled back" ($afterBoom -eq $beforeBoom) ("before_boom {0} -> {1}" -f $beforeBoom, $afterBoom)
Show "the injected detail is not leaked" (-not ($res.body -match "injected")) ("body: {0}" -f $res.body)

# --- 3. The retry the client would send is now safe -------------------------
# This is the actual bug: the frontend re-queues on 5xx, so a partial write plus
# a retry duplicated rows. With nothing written, the retry is the whole batch.
Sql "DROP TRIGGER m6_boom;" | Out-Null
$before = FrontendRows
$beforeBoom = [int]$((Sql "SELECT COUNT(*) FROM audit_logs WHERE action='before_boom';") -replace "\D", "")
$beforeMiddle = [int]$((Sql "SELECT COUNT(*) FROM audit_logs WHERE action='second';") -replace "\D", "")
$f = Batch "second"
$res = Invoke-Curl "$base/logs/batch" "POST" $f
Remove-Item $f -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 300
$after = FrontendRows
$afterBoom = [int]$((Sql "SELECT COUNT(*) FROM audit_logs WHERE action='before_boom';") -replace "\D", "")
$afterMiddle = [int]$((Sql "SELECT COUNT(*) FROM audit_logs WHERE action='second';") -replace "\D", "")
Write-Host ("  retried batch: HTTP {0}, frontend rows {1} -> {2}" -f $res.code, $before, $after)
Show "the retried batch succeeds" ($res.code -eq 200) ("HTTP {0}" -f $res.code)
Show "the retry writes the whole batch" (($after - $before) -eq 3) ("added: {0}" -f ($after - $before))
Show "no row from the failed attempt was duplicated" ((($afterBoom - $beforeBoom) -eq 1) -and (($afterMiddle - $beforeMiddle) -eq 1)) ("before_boom +{0}, second +{1} (expected +1 each)" -f ($afterBoom - $beforeBoom), ($afterMiddle - $beforeMiddle))

# --- 4. A batch rejected by validation writes nothing -----------------------
$before = FrontendRows
$f = Text-File '{"logs":[{"level":"info","category":"ui","action":"ok"},{"level":"debug","category":"ui","action":"bad"},{"level":"info","category":"ui","action":"ok2"}]}'
$res = Invoke-Curl "$base/logs/batch" "POST" $f
Remove-Item $f -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 300
$after = FrontendRows
Write-Host ("  invalid batch: HTTP {0}, frontend rows {1} -> {2}" -f $res.code, $before, $after)
Show "an invalid entry rejects the whole batch" ($res.code -eq 400) ("HTTP {0}" -f $res.code)
Show "the rejected batch writes nothing" (($after - $before) -eq 0) ("added: {0}" -f ($after - $before))

# --- 5. The audit trail stays readable after all of it ----------------------
$rows = Invoke-RestMethod -Uri "$base/audit/logs?limit=5&source=frontend" -Method Get -Headers @{ "X-Lexio-Token" = $token }
Show "the trail is still readable" ($rows.logs.Count -gt 0) ("returned {0} row(s), total {1}" -f $rows.logs.Count, $rows.total)

Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 1
Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue

Write-Host ""
if ($fail -eq 0) {
  Write-Host "M6 PASS: batches are atomic and the client retry is safe." -ForegroundColor Green
  exit 0
} else {
  Write-Host ("M6 FAIL: {0} check(s) failed." -f $fail) -ForegroundColor Red
  exit 1
}