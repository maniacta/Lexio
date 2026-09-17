# Desktop mode reaches the backend cross-origin: the packaged app is served
# from http://tauri.localhost (Windows) or tauri://localhost (elsewhere) while
# the API lives on http://127.0.0.1:3001. Browsers require a preflight for the
# custom X-Lexio-Token header, and the preflight carries no token, so this
# exercises the whole chain that the relative-URL bug had broken.
#
# ASCII-only: Windows PowerShell 5.1 decodes BOM-less files as ANSI.
$ErrorActionPreference = "Continue"
$api = "http://127.0.0.1:3001/api"
$fail = 0

function Show($label, $ok, $detail) {
  if ($ok) {
    Write-Host ("[PASS] {0} -> {1}" -f $label, $detail) -ForegroundColor Green
  } else {
    Write-Host ("[FAIL] {0} -> {1}" -f $label, $detail) -ForegroundColor Red
    $script:fail++
  }
}

function Run-Curl($curlArgs) {
  return (& curl.exe @curlArgs) -join ""
}

$token = (Invoke-RestMethod -Uri "$api/auth/token" -Method Get).token

$origins = @(
  "http://tauri.localhost",
  "https://tauri.localhost",
  "tauri://localhost"
)

foreach ($origin in $origins) {
  Write-Host ("--- Origin: {0} ---" -f $origin)

  # 1. Preflight must succeed WITHOUT a token (browsers never send one).
  $ph = Run-Curl @("-s", "-D", "-", "-o", "NUL", "-X", "OPTIONS",
               "-H", "Origin: $origin",
               "-H", "Access-Control-Request-Method: GET",
               "-H", "Access-Control-Request-Headers: x-lexio-token",
               "$api/settings/providers")
  $code = "?"
  if ($ph -match "(?m)^HTTP/\S+ (\d+)") { $code = $Matches[1] }
  $allowed = $ph -match "Access-Control-Allow-Origin"
  Show "preflight allowed without token" ($allowed -and $code -match "^2") ("HTTP {0}" -f $code)

  $echoed = $ph -match ("Access-Control-Allow-Origin: [^\r\n]*" + [regex]::Escape($origin))
  Show "preflight echoes the desktop origin" $echoed "allow-origin contains origin"

  # 2. The real request must be accepted with origin + token.
  $rh = Run-Curl @("-s", "-o", "NUL", "-w", "%{http_code}", "-X", "GET",
               "-H", "Origin: $origin", "-H", "X-Lexio-Token: $token",
               "$api/settings/providers")
  Show "authenticated cross-origin request" ($rh -eq "200") ("HTTP {0}" -f $rh)

  # 3. The origin must not be a way around the token requirement.
  $nh = Run-Curl @("-s", "-o", "NUL", "-w", "%{http_code}", "-X", "GET",
               "-H", "Origin: $origin", "$api/settings/providers")
  Show "token still required from desktop origin" ($nh -eq "401") ("HTTP {0}" -f $nh)
}

# 4. A foreign origin must stay blocked.
$foreign = Run-Curl @("-s", "-D", "-", "-o", "NUL", "-X", "OPTIONS",
                  "-H", "Origin: http://evil.com",
                  "-H", "Access-Control-Request-Method: GET",
                  "$api/settings/providers")
Show "foreign origin gets no allow-origin" (-not ($foreign -match "Access-Control-Allow-Origin: [^\r\n]*evil\.com")) "evil.com not echoed"

Write-Host ""
if ($fail -eq 0) {
  Write-Host "DESKTOP API PASS: cross-origin desktop requests reach the backend." -ForegroundColor Green
  exit 0
} else {
  Write-Host ("DESKTOP API FAIL: {0} check(s) failed." -f $fail) -ForegroundColor Red
  exit 1
}