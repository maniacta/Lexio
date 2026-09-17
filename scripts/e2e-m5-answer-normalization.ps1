# M5 verification.
# Numeric answers must be compared as numbers, not after stripping punctuation.
# The old code removed `.` and `,` from every answer before comparing, so the
# expected answer 1.5 normalized to "15" and a user typing 15 was marked correct.
#
# The comparison is exercised through the real /api/quiz/submit route, because
# that is where it runs in production; a unit test proves the function, this
# proves the route reaches it with the submitted strings intact.
#
# ASCII-only on purpose: Windows PowerShell 5.1 decodes BOM-less files as ANSI,
# so CJK literals in this file would corrupt the parse. Non-ASCII test data is
# built from code points below instead.
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

# PowerShell 5.1 throws on 4xx/5xx, so curl with a body temp file is used.
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

# Write text to a UTF-8 (no BOM) temp file and return its path.
function Text-File($text) {
  $p = [System.IO.Path]::GetTempFileName()
  [System.IO.File]::WriteAllText($p, $text, (New-Object System.Text.UTF8Encoding($false)))
  return $p
}

# CJK test data, built from code points so this script stays ASCII-only.
$wall = [string][char]0x7EC6 + [char]0x80DE + [char]0x58C1        # "cell wall"
$membrane = [string][char]0x7EC6 + [char]0x80DE + [char]0x819C    # "cell membrane"

$server = Join-Path $env:TEMP "lexio-audit-target\debug\server.exe"
$work = Join-Path $env:TEMP "lexio-e2e-m5"
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

# --- Seed questions directly ------------------------------------------------
# No endpoint creates a question (they come from the LLM), so the fixture goes
# straight into the tables the app uses. sqlite3 reads the SQL from a UTF-8 file
# because the command line would re-encode CJK through the ANSI code page.
function Seed-Question($id, $kpId, $type, $answer) {
  # OR IGNORE on the KP: every fixture shares one parent, and a UNIQUE failure
  # would abort the batch before the question insert ran.
  $sql = "INSERT OR IGNORE INTO knowledge_points (id,title) VALUES ('$kpId','kp'); INSERT INTO quiz_questions (id,kp_id,type,question,options,answer,explanation) VALUES ('$id','$kpId','$type','q',NULL,'$answer','');"
  $file = Text-File $sql
  & sqlite3 $db ".read $file" | Out-Null
  Remove-Item $file -Force -ErrorAction SilentlyContinue
}

function Submit($questionId, $answer) {
  # The route compares exactly the strings it receives, so what is sent here is
  # what the comparison sees.
  $escaped = $answer.Replace('"', '\"')
  $f = Text-File ('{"question_id":"' + $questionId + '","user_answer":"' + $escaped + '"}')
  $res = Invoke-Curl "$base/quiz/submit" "POST" $f
  Remove-Item $f -Force -ErrorAction SilentlyContinue
  return $res
}

function Verdict($res) {
  # A 404 would mean the fixture is missing, which must not read as "incorrect".
  if ($res.code -ne 200) { return "HTTP $($res.code) $($res.body)" }
  if ($res.body -match '"is_correct":\s*true') { return "correct" }
  return "incorrect"
}

# The seed must be visible before any negative result means anything.
Seed-Question "q-dec" "kp1" "fill_blank" "1.5"
$seeded = (& sqlite3 $db "SELECT COUNT(*) FROM quiz_questions;" 2>&1) -join ""
Write-Host ("  question count after seed: {0}" -f $seeded)
Show "the fixture is visible in the database" (([int]$seeded) -eq 1) ("rows: {0}" -f $seeded)

# --- 1. The regression: 1.5 expected, 15 submitted --------------------------
$res = Submit "q-dec" "15"
$v = Verdict $res
Write-Host ("  1.5 vs 15: {0}" -f $v)
Show "15 is NOT accepted for an expected 1.5" ($v -eq "incorrect") ("verdict: {0}" -f $v)

# --- 2. The reverse direction: 15 expected, 1.5 submitted -------------------
Seed-Question "q-int" "kp1" "fill_blank" "15"
$res = Submit "q-int" "1.5"
$v = Verdict $res
Show "1.5 is NOT accepted for an expected 15" ($v -eq "incorrect") ("verdict: {0}" -f $v)

# --- 3. The correct decimal still passes -----------------------------------
$res = Submit "q-dec" "1.5"
$v = Verdict $res
Show "the exact decimal 1.5 is still accepted" ($v -eq "correct") ("verdict: {0}" -f $v)

# --- 4. Equivalent spellings and separators still pass ----------------------
Seed-Question "q-trail" "kp1" "fill_blank" "1.50"
$res = Submit "q-trail" "1.5"
$v = Verdict $res
Show "1.50 and 1.5 are the same number" ($v -eq "correct") ("verdict: {0}" -f $v)

Seed-Question "q-thousand" "kp1" "fill_blank" "1,000"
$res = Submit "q-thousand" "1000"
$v = Verdict $res
Show "1,000 and 1000 are the same number" ($v -eq "correct") ("verdict: {0}" -f $v)

# --- 5. A comma decimal is not a thousands separator ------------------------
# The old code stripped the comma, so 3,14 matched 314. `.` used to be stripped
# too, so it also matched 3.14 by accident; that accident is now gone as well.
$res = Submit "q-thousand" "3,14"
$v = Verdict $res
Show "3,14 is not read as 314" ($v -eq "incorrect") ("verdict: {0}" -f $v)

# --- 6. Text answers keep their old tolerance -------------------------------
Seed-Question "q-text" "kp1" "fill_blank" $wall
$quoted = '"' + $wall + '"' + [char]0x3002
$res = Submit "q-text" $quoted
$v = Verdict $res
Show "a quoted, punctuated text answer still matches" ($v -eq "correct") ("verdict: {0}" -f $v)

# --- 7. A wrong text answer is still wrong ---------------------------------
$res = Submit "q-text" $membrane
$v = Verdict $res
Show "a different text answer is still wrong" ($v -eq "incorrect") ("verdict: {0}" -f $v)

# --- 8. Attempts are recorded with their verdict ----------------------------
$attemptCount = (& sqlite3 $db "SELECT COUNT(*) FROM quiz_attempts;" 2>&1) -join ""
$correctCount = (& sqlite3 $db "SELECT COUNT(*) FROM quiz_attempts WHERE is_correct=1;" 2>&1) -join ""
Write-Host ("  attempts recorded: {0} (correct: {1})" -f $attemptCount, $correctCount)
Show "every submission is recorded" (([int]$attemptCount) -eq 8) ("rows: {0}" -f $attemptCount)
Show "recorded verdicts match the checks" (([int]$correctCount) -eq 4) ("correct rows: {0}" -f $correctCount)

# --- 9. Oversized answers are still bounded (M2 must not regress) -----------
$f = Text-File ('{"question_id":"q-dec","user_answer":"' + ("a" * 5000) + '"}')
$res = Invoke-Curl "$base/quiz/submit" "POST" $f
Remove-Item $f -Force -ErrorAction SilentlyContinue
Show "the M2 answer limit still holds" ($res.code -eq 400) ("HTTP {0}" -f $res.code)

Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 1
Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue

Write-Host ""
if ($fail -eq 0) {
  Write-Host "M5 PASS: numeric answers compare as numbers; text tolerance intact." -ForegroundColor Green
  exit 0
} else {
  Write-Host ("M5 FAIL: {0} check(s) failed." -f $fail) -ForegroundColor Red
  exit 1
}
