# M7 verification.
# Occupying 3001 must exit with a readable PORT_IN_USE message, not an
# unwrap panic. Freeing the port must still let the backend boot.
#
# ASCII-only on purpose: Windows PowerShell 5.1 decodes BOM-less files as ANSI.
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
$work = Join-Path $env:TEMP "lexio-e2e-m7"
$stdout = Join-Path $work "stdout.txt"
$stderr = Join-Path $work "stderr.txt"

Get-Process -Name server -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 2
Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $work | Out-Null

if (-not (Test-Path $server)) {
  Show "server binary present" $false $server
  exit 1
}

$holder = $null
try {
  $holder = New-Object System.Net.Sockets.TcpListener ([System.Net.IPAddress]::Loopback, 3001)
  $holder.Start()
  Show "occupied 3001 for the test" $true "TcpListener held"
} catch {
  Show "occupied 3001 for the test" $false $_.Exception.Message
  exit 1
}

$env:LEXIO_MASTER_KEY_STORE = "file"
$proc = Start-Process -FilePath $server -WorkingDirectory $work -PassThru -WindowStyle Hidden `
  -RedirectStandardOutput $stdout -RedirectStandardError $stderr
$waited = $proc.WaitForExit(15000)
$still = Get-Process -Id $proc.Id -ErrorAction SilentlyContinue
if (-not $waited -or $still) {
  if ($still) { Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue }
  Show "occupied-port process exits" $false "still running after 15s"
} else {
  Show "occupied-port process exits" $true "stopped after bind failure"
}

function Read-Utf8($path) {
  if (-not (Test-Path $path)) { return "" }
  return [System.IO.File]::ReadAllText($path, [System.Text.Encoding]::UTF8)
}
$combined = (Read-Utf8 $stdout) + (Read-Utf8 $stderr)

Show "message contains PORT_IN_USE" ($combined.Contains("PORT_IN_USE")) ("len={0}" -f $combined.Length)
$preview = $combined.Substring(0, [Math]::Min(180, $combined.Length)).Replace("`r", " ").Replace("`n", " ")
Show "message is not an unwrap panic" (-not ($combined.Contains("unwrap()") -or $combined.Contains("panicked at"))) ("preview={0}" -f $preview)

try { $holder.Stop() } catch {}
$holder = $null
Start-Sleep -Seconds 1

$bootOut = Join-Path $work "boot-stdout.txt"
$bootErr = Join-Path $work "boot-stderr.txt"
$boot = Start-Process -FilePath $server -WorkingDirectory $work -PassThru -WindowStyle Hidden `
  -RedirectStandardOutput $bootOut -RedirectStandardError $bootErr
Start-Sleep -Seconds 5

$token = $null
for ($i = 0; $i -lt 10 -and -not $token; $i++) {
  try { $token = (Invoke-RestMethod -Uri "http://127.0.0.1:3001/api/auth/token" -Method Get).token } catch { Start-Sleep -Milliseconds 700 }
}
Show "backend boots after port is freed" ([bool]$token) $(if ($token) { "token acquired" } else { "no token" })

if ($token) {
  try {
    $health = Invoke-WebRequest -Uri "http://127.0.0.1:3001/api/health" -Method Get -UseBasicParsing
    Show "health still open" ($health.StatusCode -eq 200) ("HTTP {0}" -f $health.StatusCode)
  } catch {
    Show "health still open" $false $_.Exception.Message
  }
}

Stop-Process -Id $boot.Id -Force -ErrorAction SilentlyContinue
Get-Process -Name server -ErrorAction SilentlyContinue | Stop-Process -Force

Write-Host ""
if ($fail -eq 0) {
  Write-Host "M7 PASS: occupied 3001 exits with PORT_IN_USE; free port still boots." -ForegroundColor Green
  exit 0
} else {
  Write-Host "M7 FAIL: $fail check(s) failed." -ForegroundColor Red
  exit 1
}
