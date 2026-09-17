# M8 verification.
# SM-2 must advance at most once per *local* calendar day. The UTC-day gate
# let a UTC+8 user reviewing at 07:00 and 09:00 local (23:00Z / 01:00Z) take
# two steps in one morning. The boundary itself is covered by unit tests with
# a FixedOffset clock; this script checks the live route still applies the
# gate: two update_mastery calls in a row must not double the interval.
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
$work = Join-Path $env:TEMP "lexio-e2e-m8"

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

$f = Text-File '{"title":"sm2 kp","summary":"s","content":"c","tags":[],"source_ids":[]}'
$res = Invoke-Curl "$base/knowledge" "POST" $f
Remove-Item $f -Force -ErrorAction SilentlyContinue
$kp = $null
if ($res.body -match '"id"\s*:\s*"([^"]+)"') { $kp = $Matches[1] }
Show "knowledge point created" (($res.code -eq 201) -and $kp) ("id={0}" -f $kp)

function Mastery($correct) {
  $f = Text-File ('{"kp_id":"' + $kp + '","is_correct":' + $correct + '}')
  $r = Invoke-Curl "$base/ai/update-mastery" "POST" $f
  Remove-Item $f -Force -ErrorAction SilentlyContinue
  return $r
}

$first = Mastery "true"
Show "first review is 200" ($first.code -eq 200) ("HTTP {0}" -f $first.code)
$rep1 = 0; $int1 = 0
if ($first.body -match '"repetitions"\s*:\s*(\d+)') { $rep1 = [int]$Matches[1] }
if ($first.body -match '"interval_days"\s*:\s*(\d+)') { $int1 = [int]$Matches[1] }
Write-Host ("  first review: repetitions={0} interval_days={1}" -f $rep1, $int1)
Show "the first review advances SM-2" (($rep1 -ge 1) -and ($int1 -ge 1)) ("rep={0} interval={1}" -f $rep1, $int1)

$second = Mastery "true"
Show "second review is 200" ($second.code -eq 200) ("HTTP {0}" -f $second.code)
$rep2 = 0; $int2 = 0
if ($second.body -match '"repetitions"\s*:\s*(\d+)') { $rep2 = [int]$Matches[1] }
if ($second.body -match '"interval_days"\s*:\s*(\d+)') { $int2 = [int]$Matches[1] }
Write-Host ("  second review: repetitions={0} interval_days={1}" -f $rep2, $int2)
Show "the same local day does not take a second SM-2 step" (($rep2 -eq $rep1) -and ($int2 -eq $int1)) ("rep {0}->{1}, interval {2}->{3}" -f $rep1, $rep2, $int1, $int2)

# A wrong answer the same day must also not rewrite the record (the UTC-day
# bug made some same-local-day fails look like a new day and still skip, or
# the other way around). Either way the stored repetitions stay put.
$failAns = Mastery "false"
$rep3 = 0
if ($failAns.body -match '"repetitions"\s*:\s*(\d+)') { $rep3 = [int]$Matches[1] }
Show "a same-day fail does not clobber the first step" ($rep3 -eq $rep1) ("rep still {0}" -f $rep3)

Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 1
Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue

Write-Host ""
if ($fail -eq 0) {
  Write-Host "M8 PASS: SM-2 advances at most once per local day." -ForegroundColor Green
  exit 0
} else {
  Write-Host ("M8 FAIL: {0} check(s) failed." -f $fail) -ForegroundColor Red
  exit 1
}
