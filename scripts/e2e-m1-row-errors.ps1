# M1 verification.
# A row that cannot be mapped used to vanish: list endpoints returned a shorter
# page with HTTP 200, and single-row getters returned "not found". A corrupt
# column must now be a 500, and a well-formed list must still return every row.
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

function Text-File($text) {
  $p = [System.IO.Path]::GetTempFileName()
  [System.IO.File]::WriteAllText($p, $text, (New-Object System.Text.UTF8Encoding($false)))
  return $p
}

$server = Join-Path $env:TEMP "lexio-audit-target\debug\server.exe"
$work = Join-Path $env:TEMP "lexio-e2e-m1"
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

# --- 1. A well-formed list still returns every row --------------------------
$f = Text-File '{"title":"ok source","type":"text","content":"hello","tags":[],"origin":"user"}'
$res = Invoke-Curl "$base/sources" "POST" $f
Remove-Item $f -Force -ErrorAction SilentlyContinue
Show "a legal source is created" ($res.code -eq 201) ("HTTP {0}" -f $res.code)

$res = Invoke-Curl "$base/sources" "GET" $null
$listed = 0
if ($res.body -match '"title"') { $listed = 1 }
Show "listing a clean table is 200" ($res.code -eq 200) ("HTTP {0}" -f $res.code)
Show "the created source is in the list" ($listed -eq 1) ("body starts: {0}" -f $res.body.Substring(0, [Math]::Min(80, $res.body.Length)))

# --- 2. A corrupt row must not shrink the list ------------------------------
# hidden is INTEGER; a non-numeric TEXT value fails the i32 mapping that
# source_from_row performs. Previously this row was skipped.
$sql = Text-File "INSERT INTO sources (id, title, type, content, tags, origin, hidden) VALUES ('bad','t','text','c','[]','user','not-an-int');"
& sqlite3 $db ".read $sql" | Out-Null
Remove-Item $sql -Force -ErrorAction SilentlyContinue
$count = (& sqlite3 $db "SELECT COUNT(*) FROM sources;" 2>&1) -join ""
Show "both rows are in the table" ($count -eq "2") ("rows: {0}" -f $count)

$res = Invoke-Curl "$base/sources?include_hidden=true" "GET" $null
Write-Host ("  list with corrupt row: HTTP {0} body {1}" -f $res.code, $res.body.Substring(0, [Math]::Min(80, $res.body.Length)))
Show "a mapping failure is a 500, not a shorter 200" ($res.code -eq 500) ("HTTP {0}" -f $res.code)
Show "the 500 does not leak the rusqlite type error" (-not ($res.body -match "InvalidColumnType|not-an-int|INTEGER")) ("body: {0}" -f $res.body)

# --- 3. A single-row get must not look like not-found -----------------------
$res = Invoke-Curl "$base/sources/bad" "GET" $null
Write-Host ("  get corrupt id: HTTP {0}" -f $res.code)
Show "get_source on a corrupt row is 500, not 404" ($res.code -eq 500) ("HTTP {0}" -f $res.code)

# --- 4. Knowledge list still works on a clean table -------------------------
$f = Text-File '{"title":"ok kp","summary":"s","content":"c","tags":[],"source_ids":[]}'
$res = Invoke-Curl "$base/knowledge" "POST" $f
Remove-Item $f -Force -ErrorAction SilentlyContinue
Show "a legal knowledge point is created" ($res.code -eq 201) ("HTTP {0}" -f $res.code)
$res = Invoke-Curl "$base/knowledge" "GET" $null
Show "listing knowledge points is 200" ($res.code -eq 200) ("HTTP {0}" -f $res.code)

Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 1
Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue

Write-Host ""
if ($fail -eq 0) {
  Write-Host "M1 PASS: a corrupt row is an error; a clean list is complete." -ForegroundColor Green
  exit 0
} else {
  Write-Host ("M1 FAIL: {0} check(s) failed." -f $fail) -ForegroundColor Red
  exit 1
}
