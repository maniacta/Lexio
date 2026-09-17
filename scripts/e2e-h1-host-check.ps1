# H1 verification: local requests must work, LAN/dns-rebind Hosts must be blocked.
# The backend trusts loopback peers, so the Host header is the control under test.
$ErrorActionPreference = "Continue"
$base = "http://127.0.0.1:3001/api"
$fail = 0

function Show($label, $ok, $detail) {
  $tag = if ($ok) { "PASS" } else { "FAIL"; $script:fail++ }
  $color = if ($ok) { "Green" } else { "Red" }
  Write-Host ("[{0}] {1} -> {2}" -f $tag, $label, $detail) -ForegroundColor $color
}

# 1. Normal local bootstrap (Host: 127.0.0.1) must still succeed.
try {
  $r = Invoke-WebRequest -Uri "$base/auth/token" -Method Get -UseBasicParsing
  $token = ($r.Content | ConvertFrom-Json).token
  Show "local GET /auth/token (Host: 127.0.0.1)" ($r.StatusCode -eq 200) "HTTP $($r.StatusCode), token len $($token.Length)"
} catch {
  Show "local GET /auth/token (Host: 127.0.0.1)" $false $_.Exception.Message
  exit 1
}

# 2. Attacker on the LAN reaching the backend through the dev proxy.
#    ConnectInfo says 127.0.0.1, but Host carries the attacker's address.
$lanHosts = @("192.168.0.42:14200", "attacker.example.com", "my-laptop.local:14200")
foreach ($h in $lanHosts) {
  try {
    $resp = Invoke-WebRequest -Uri "$base/auth/token" -Method Get -UseBasicParsing -Headers @{ Host = $h }
    Show "LAN Host '$h' blocked" $false "LEAKED token (HTTP $($resp.StatusCode))"
  } catch {
    $code = $_.Exception.Response.StatusCode.value__
    Show "LAN Host '$h' blocked" ($code -eq 403) "HTTP $code"
  }
}

# 3. DNS-rebinding: Host points at an attacker domain while connecting to loopback.
try {
  $resp = Invoke-WebRequest -Uri "$base/settings" -Method Get -UseBasicParsing -Headers @{ Host = "rebind.evil.com" }
  Show "dns-rebind Host blocked (authenticated route)" $false "reached route (HTTP $($resp.StatusCode))"
} catch {
  $code = $_.Exception.Response.StatusCode.value__
  Show "dns-rebind Host blocked (authenticated route)" ($code -eq 403) "HTTP $code"
}

# 4. A foreign Host must be blocked even WITH a valid token (defence in depth).
$headers = @{ "X-Lexio-Token" = $token; Host = "10.0.0.5:3001" }
try {
  $resp = Invoke-WebRequest -Uri "$base/chat/sessions" -Method Get -UseBasicParsing -Headers $headers
  Show "foreign Host blocked despite valid token" $false "HTTP $($resp.StatusCode)"
} catch {
  $code = $_.Exception.Response.StatusCode.value__
  Show "foreign Host blocked despite valid token" ($code -eq 403) "HTTP $code"
}

# 5. Legitimate local use must be unaffected: authenticated route with local Host.
try {
  $okHeaders = @{ "X-Lexio-Token" = $token }
  $resp = Invoke-WebRequest -Uri "$base/chat/sessions" -Method Get -UseBasicParsing -Headers $okHeaders
  Show "authenticated local request unaffected" ($resp.StatusCode -eq 200) "HTTP $($resp.StatusCode)"
} catch {
  Show "authenticated local request unaffected" $false $_.Exception.Message
}

# 6. Missing token still 401 (auth still enforced), and health is open.
try {
  $resp = Invoke-WebRequest -Uri "$base/chat/sessions" -Method Get -UseBasicParsing
  Show "no-token still rejected" $false "HTTP $($resp.StatusCode)"
} catch {
  Show "no-token still rejected" ($_.Exception.Response.StatusCode.value__ -eq 401) "HTTP $($_.Exception.Response.StatusCode.value__)"
}
try {
  $resp = Invoke-WebRequest -Uri "$base/health" -Method Get -UseBasicParsing
  Show "health endpoint still open" ($resp.StatusCode -eq 200) "HTTP $($resp.StatusCode)"
} catch {
  Show "health endpoint still open" $false $_.Exception.Message
}

Write-Host ""
if ($fail -eq 0) {
  Write-Host "H1 PASS: LAN hosts can no longer obtain the token or reach any route." -ForegroundColor Green
  exit 0
} else {
  Write-Host "H1 FAIL: $fail check(s) failed." -ForegroundColor Red
  exit 1
}
