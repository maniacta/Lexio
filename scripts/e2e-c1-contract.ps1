# C1 end-to-end contract check.
# Reproduces the exact flow the frontend performs on session reload:
#   create session -> append assistant message with actions -> GET messages
# PASS only if `actions` comes back as a real JSON array (not a string).
$ErrorActionPreference = "Stop"
$base = "http://127.0.0.1:3001/api"

$token = (Invoke-RestMethod -Uri "$base/auth/token" -Method Get).token
Write-Host "1. got token: $($token.Substring(0,8))..."

$headers = @{ "X-Lexio-Token" = $token; "Content-Type" = "application/json" }

$session = Invoke-RestMethod -Uri "$base/chat/sessions" -Method Post `
  -Headers $headers -Body '{"title":"C1 contract test"}'
Write-Host "2. created session: $($session.id)"

# Same shape the UI posts: actions/context are stringified by the client.
$appendBody = @{
  session_id = $session.id
  role       = "assistant"
  content    = "I listed 2 knowledge points."
  actions    = '[{"type":"navigate_learning","label":"Rust ownership","payload":{"kpId":"k1","kpTitle":"Rust ownership"}}]'
  context    = '{"plan":{"id":"p1"}}'
} | ConvertTo-Json -Compress

$appended = Invoke-RestMethod -Uri "$base/chat/messages" -Method Post `
  -Headers $headers -Body $appendBody
Write-Host "3. append_message response actions type: $($appended.actions.GetType().Name)"

# ---- The regression assertion ----
$msgs = Invoke-RestMethod -Uri "$base/chat/sessions/$($session.id)/messages" -Method Get -Headers $headers
$actions = $msgs[0].actions

Write-Host "4. GET messages -> actions .NET type: $($actions.GetType().FullName)"

if ($actions -is [string]) {
  Write-Host "FAIL: actions came back as a STRING -> frontend would call .map() and crash" -ForegroundColor Red
  exit 1
}
if ($actions -isnot [System.Object[]]) {
  Write-Host "FAIL: actions is not an array (got $($actions.GetType().Name))" -ForegroundColor Red
  exit 1
}
if ($actions[0].type -ne "navigate_learning") {
  Write-Host "FAIL: action type lost in round-trip: $($actions[0].type)" -ForegroundColor Red
  exit 1
}
if ($actions[0].payload.kpId -ne "k1") {
  Write-Host "FAIL: payload.kpId lost in round-trip: $($actions[0].payload.kpId)" -ForegroundColor Red
  exit 1
}
if ($msgs[0].context.plan.id -ne "p1") {
  Write-Host "FAIL: context.plan.id lost in round-trip" -ForegroundColor Red
  exit 1
}
Write-Host "   action[0].type   = $($actions[0].type)"
Write-Host "   action[0].payload = kpId=$($actions[0].payload.kpId)"
Write-Host "   context.plan.id  = $($msgs[0].context.plan.id)"

# Also assert append_message itself already returns parsed JSON.
if ($appended.actions -is [string]) {
  Write-Host "FAIL: append_message still returns a string actions field" -ForegroundColor Red
  exit 1
}
Write-Host "5. append_message returns parsed JSON too"

# A user message with no actions must serialize as null, not "null".
$userSession = Invoke-RestMethod -Uri "$base/chat/sessions" -Method Post `
  -Headers $headers -Body '{"title":"C1 null test"}'
$userBody = @{
  session_id = $userSession.id
  role       = "user"
  content    = "hello"
} | ConvertTo-Json -Compress
Invoke-RestMethod -Uri "$base/chat/messages" -Method Post -Headers $headers -Body $userBody | Out-Null
$userMsgs = Invoke-RestMethod -Uri "$base/chat/sessions/$($userSession.id)/messages" -Method Get -Headers $headers
if ($null -ne $userMsgs[0].actions) {
  Write-Host "FAIL: absent actions should be null, got: $($userMsgs[0].actions)" -ForegroundColor Red
  exit 1
}
Write-Host "6. absent actions serializes as null"

# Clean up the throwaway sessions.
Invoke-RestMethod -Uri "$base/chat/sessions/$($session.id)" -Method Delete -Headers $headers | Out-Null
Invoke-RestMethod -Uri "$base/chat/sessions/$($userSession.id)" -Method Delete -Headers $headers | Out-Null

Write-Host "`nC1 PASS: API contract now hands the client real JSON arrays." -ForegroundColor Green
