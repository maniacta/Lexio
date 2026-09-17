# M12 verification.
# Hidden sources must not appear in search. list_sources already filtered
# `hidden = 0`; search_sources did not, so a hidden row still ranked.
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
$work = Join-Path $env:TEMP "lexio-e2e-m12"

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

function Create-Source($title) {
  $f = Text-File ('{"title":"' + $title + '","type":"text","content":"ETag cache marker","tags":[],"origin":"user"}')
  $res = Invoke-Curl "$base/sources" "POST" $f
  Remove-Item $f -Force -ErrorAction SilentlyContinue
  $id = $null
  if ($res.body -match '"id"\s*:\s*"([^"]+)"') { $id = $Matches[1] }
  return @{ code = $res.code; id = $id; body = $res.body }
}

$visible = Create-Source "visible cache doc"
$hidden = Create-Source "hidden cache doc"
Show "two matching sources created" (($visible.code -eq 201) -and ($hidden.code -eq 201) -and $visible.id -and $hidden.id) ("visible={0} hidden={1}" -f $visible.id, $hidden.id)

$f = Text-File '{"hidden":true}'
$res = Invoke-Curl "$base/sources/$($hidden.id)/hide" "POST" $f
Remove-Item $f -Force -ErrorAction SilentlyContinue
Show "the second source is hidden" ($res.code -eq 200) ("HTTP {0}" -f $res.code)

$res = Invoke-Curl "$base/sources?search=cache" "GET" $null
Write-Host ("  search body: {0}" -f $res.body.Substring(0, [Math]::Min(200, $res.body.Length)))
Show "search is 200" ($res.code -eq 200) ("HTTP {0}" -f $res.code)
Show "the visible source is still found" ($res.body -match [regex]::Escape($visible.id)) ("visible id present")
Show "the hidden source is not found" (-not ($res.body -match [regex]::Escape($hidden.id))) ("hidden id absent")

$res = Invoke-Curl "$base/sources?include_hidden=true" "GET" $null
Show "include_hidden still lists the hidden row" ($res.body -match [regex]::Escape($hidden.id)) ("list-all includes hidden")
$res = Invoke-Curl "$base/sources" "GET" $null
Show "the default list still omits the hidden row" (-not ($res.body -match [regex]::Escape($hidden.id))) ("default list omits hidden")

Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 1
Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue

Write-Host ""
if ($fail -eq 0) {
  Write-Host "M12 PASS: hidden sources are omitted from search." -ForegroundColor Green
  exit 0
} else {
  Write-Host ("M12 FAIL: {0} check(s) failed." -f $fail) -ForegroundColor Red
  exit 1
}
