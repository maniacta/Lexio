# M13 verification.
# Session titles used to live only in React state. set_session_title had no
# HTTP route and no frontend caller, so a reload showed every session as
# "new conversation" (the create-time default).
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
$work = Join-Path $env:TEMP "lexio-e2e-m13"
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

# --- 1. Create a session; default title is the create-time value ------------
$f = Text-File '{"title":"new chat"}'
$res = Invoke-Curl "$base/chat/sessions" "POST" $f
Remove-Item $f -Force -ErrorAction SilentlyContinue
$id = $null
if ($res.body -match '"id"\s*:\s*"([^"]+)"') { $id = $Matches[1] }
Show "session created" (($res.code -eq 201) -and $id) ("HTTP {0} id={1}" -f $res.code, $id)

$res = Invoke-Curl "$base/chat/sessions" "GET" $null
Show "list shows the create-time title" ($res.body -match "new chat") ("list contains create title")

# --- 2. Persist a new title -------------------------------------------------
$f = Text-File '{"title":"I want to learn Rust"}'
$res = Invoke-Curl "$base/chat/sessions/$id/title" "POST" $f
Remove-Item $f -Force -ErrorAction SilentlyContinue
Show "title update is 200" ($res.code -eq 200) ("HTTP {0}" -f $res.code)

$res = Invoke-Curl "$base/chat/sessions" "GET" $null
Show "the list now carries the persisted title" ($res.body -match "I want to learn Rust") ("list contains new title")
Show "the old title is gone" (-not ($res.body -match "new chat")) ("create-time title absent")

$dbTitle = (& sqlite3 $db "SELECT title FROM chat_sessions WHERE id='$id';" 2>&1) -join ""
Show "the title is in sqlite" ($dbTitle -eq "I want to learn Rust") ("db: {0}" -f $dbTitle)

# --- 3. An oversized title is still a 400 (M2 must not regress) -------------
$f = Text-File ('{"title":"' + ("t" * 300) + '"}')
$res = Invoke-Curl "$base/chat/sessions/$id/title" "POST" $f
Remove-Item $f -Force -ErrorAction SilentlyContinue
Show "a 300-char title is refused" ($res.code -eq 400) ("HTTP {0}" -f $res.code)
$dbTitle = (& sqlite3 $db "SELECT title FROM chat_sessions WHERE id='$id';" 2>&1) -join ""
Show "a refused title does not overwrite" ($dbTitle -eq "I want to learn Rust") ("db still: {0}" -f $dbTitle)

# --- 4. The route is token-protected ----------------------------------------
$f = Text-File '{"title":"x"}'
$tmp = [System.IO.Path]::GetTempFileName()
$code = (& curl.exe -s -o $tmp -w "%{http_code}" -X POST -H "Content-Type: application/json" --data-binary "@$f" "$base/chat/sessions/$id/title") -join ""
Remove-Item $tmp, $f -Force -ErrorAction SilentlyContinue
Show "title update without a token is 401" ([int]$code -eq 401) ("HTTP {0}" -f $code)

Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 1
Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue

Write-Host ""
if ($fail -eq 0) {
  Write-Host "M13 PASS: session titles persist across list/reload." -ForegroundColor Green
  exit 0
} else {
  Write-Host ("M13 FAIL: {0} check(s) failed." -f $fail) -ForegroundColor Red
  exit 1
}
