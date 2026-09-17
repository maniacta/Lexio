# L7 verification.
# Missing ids used to return 204 (delete_kp), 200 (toggle_hidden), or 500
# (delete_model via rusqlite QueryReturnedNoRows). All three must now be 404.
# Existing rows keep their previous success codes (204 / 200 / 204).
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
$work = Join-Path $env:TEMP "lexio-e2e-l7"

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

# --- missing ids are 404 -----------------------------------------------------
$res = Invoke-Curl "$base/knowledge/no-such-kp" "DELETE" $null
Show "delete missing kp is 404" ($res.code -eq 404) ("HTTP {0} body={1}" -f $res.code, $res.body)
Show "delete missing kp names the resource" ($res.body -match "not found") $res.body

$f = Text-File '{"hidden":true}'
$res = Invoke-Curl "$base/sources/no-such-source/hide" "POST" $f
Remove-Item $f -Force -ErrorAction SilentlyContinue
Show "hide missing source is 404" ($res.code -eq 404) ("HTTP {0} body={1}" -f $res.code, $res.body)
Show "hide missing source names the resource" ($res.body -match "not found") $res.body

$res = Invoke-Curl "$base/settings/providers/no-such-p/models/no-such-m" "DELETE" $null
Show "delete missing model is 404 not 500" ($res.code -eq 404) ("HTTP {0} body={1}" -f $res.code, $res.body)
Show "delete missing model names the resource" ($res.body -match "not found") $res.body

# --- existing rows keep success codes ----------------------------------------
$f = Text-File '{"title":"l7 kp","summary":"s","content":"c","tags":[],"source_ids":[]}'
$res = Invoke-Curl "$base/knowledge" "POST" $f
Remove-Item $f -Force -ErrorAction SilentlyContinue
$kpid = $null
if ($res.body -match '"id"\s*:\s*"([^"]+)"') { $kpid = $Matches[1] }
Show "kp created" (($res.code -eq 201) -and $kpid) ("HTTP {0} id={1}" -f $res.code, $kpid)

$res = Invoke-Curl "$base/knowledge/$kpid" "DELETE" $null
Show "delete existing kp is 204" ($res.code -eq 204) ("HTTP {0}" -f $res.code)
$res = Invoke-Curl "$base/knowledge/$kpid" "DELETE" $null
Show "second delete of same kp is 404" ($res.code -eq 404) ("HTTP {0} body={1}" -f $res.code, $res.body)

$f = Text-File '{"title":"l7 source","type":"text","content":"hello","tags":[],"origin":"user"}'
$res = Invoke-Curl "$base/sources" "POST" $f
Remove-Item $f -Force -ErrorAction SilentlyContinue
$sid = $null
if ($res.body -match '"id"\s*:\s*"([^"]+)"') { $sid = $Matches[1] }
Show "source created" (($res.code -eq 201) -and $sid) ("HTTP {0} id={1}" -f $res.code, $sid)

$f = Text-File '{"hidden":true}'
$res = Invoke-Curl "$base/sources/$sid/hide" "POST" $f
Remove-Item $f -Force -ErrorAction SilentlyContinue
Show "hide existing source is 200" ($res.code -eq 200) ("HTTP {0}" -f $res.code)

$plist = Invoke-Curl "$base/settings/providers" "GET" $null
Show "providers listed" ($plist.code -eq 200) ("HTTP {0}" -f $plist.code)
$providerId = $null
$modelId = $null
try {
  $parsed = $plist.body | ConvertFrom-Json
  foreach ($p in @($parsed)) {
    $ms = @($p.models)
    if ($ms.Count -gt 0 -and $ms[0].id) {
      $providerId = [string]$p.id
      $modelId = [string]$ms[0].id
      break
    }
  }
} catch { }
Show "preset model located" ([bool]$providerId -and [bool]$modelId) ("provider={0} model={1}" -f $providerId, $modelId)

if ($providerId -and $modelId) {
  $res = Invoke-Curl "$base/settings/providers/$providerId/models/$modelId" "DELETE" $null
  Show "delete existing model is 204" ($res.code -eq 204) ("HTTP {0} body={1}" -f $res.code, $res.body)
  $res = Invoke-Curl "$base/settings/providers/$providerId/models/$modelId" "DELETE" $null
  Show "second delete of same model is 404" ($res.code -eq 404) ("HTTP {0} body={1}" -f $res.code, $res.body)
}

Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
if ($fail -gt 0) { exit 1 }
Write-Host "L7 e2e passed" -ForegroundColor Green
exit 0
