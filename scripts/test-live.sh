#!/usr/bin/env bash

# Exercise only real local HTTP. No browser scripts or Rust test runner are used.
base_url=http://127.0.0.1:5656
work_dir=
server_pid=
monitor_pid=
response_code=
headers=
body=
request_label=

fail() {
  printf 'FAIL: %s\n' "$*" >&2
  exit 1
}

# Called by the EXIT trap through cleanup.
# shellcheck disable=SC2329
stop_owned_server() {
  local attempt
  [[ -n "$server_pid" ]] || return 0
  # setsid gives just and its children a process group owned by this harness.
  if kill -0 -- "-$server_pid" 2>/dev/null; then
    kill -TERM -- "-$server_pid" 2>/dev/null
    for ((attempt = 0; attempt < 50; attempt++)); do
      kill -0 -- "-$server_pid" 2>/dev/null || break
      sleep 0.1
    done
    if kill -0 -- "-$server_pid" 2>/dev/null; then
      kill -KILL -- "-$server_pid" 2>/dev/null || return 1
    fi
  fi
  # A signal exit is expected when this harness stops its own server.
  if wait "$server_pid" 2>/dev/null; then :; fi
  server_pid=
  return 0
}

# Called by the EXIT trap registered in main.
# shellcheck disable=SC2329
cleanup() {
  local status=$?
  trap - EXIT
  trap '' TERM INT HUP
  if ! stop_owned_server; then
    printf '%s\n' 'FAIL: could not stop the owned server process group' >&2
    status=1
  fi
  if [[ -n "$monitor_pid" ]]; then
    if kill -0 "$monitor_pid" 2>/dev/null; then
      kill -TERM "$monitor_pid" 2>/dev/null
    fi
    if wait "$monitor_pid" 2>/dev/null; then :; fi
  fi
  if ((status != 0)) && [[ -f "$work_dir/server.log" ]]; then
    printf '%s\n' '--- owned just dev log (last 100 lines) ---' >&2
    tail -n 100 "$work_dir/server.log" >&2
  fi
  if [[ -n "$work_dir" ]]; then
    if ! rm -rf -- "$work_dir"; then
      printf 'FAIL: could not remove test directory %s\n' "$work_dir" >&2
      status=1
    fi
  fi
  exit "$status"
}

require_pattern() {
  rg --ignore-case --quiet -- "$1" "$2" || fail "$request_label: $3"
}

reject_pattern() {
  local status
  rg --ignore-case --quiet -- "$1" "$2"
  status=$?
  case "$status" in
    0) fail "$request_label: $3" ;;
    1) return 0 ;;
    *) fail "$request_label: could not inspect response ($status)" ;;
  esac
}

request() {
  local method=$2 path=$3
  request_label=$1
  shift 3
  headers="$work_dir/$request_label.headers"
  body="$work_dir/$request_label.body"
  response_code=$(curl --disable --silent --show-error --noproxy '*' \
    --proto '=http' --connect-timeout 2 --max-time 10 \
    --request "$method" --dump-header "$headers" --output "$body" \
    --write-out '%{http_code}' "$@" "$base_url$path") ||
    fail "$request_label: HTTP request failed"
}

require_status() {
  [[ "$response_code" == "$1" ]] ||
    fail "$request_label: expected HTTP $1, received $response_code"
}

require_cookie() {
  local cookie_name=$1 line cookie_line='' cookie_count=0 attribute pattern
  require_pattern "^set-cookie: $1=$2([;[:space:]]|$)" "$headers" "missing $1=$2 cookie"
  while IFS= read -r line || [[ -n "$line" ]]; do
    line=${line%$'\r'}
    if [[ "${line,,}" == "set-cookie: $cookie_name="* ]]; then
      cookie_line=${line,,}
      ((cookie_count += 1))
    fi
  done <"$headers"
  ((cookie_count == 1)) || fail "$request_label: expected one $cookie_name cookie line"
  for attribute in 'path=/' 'max-age=2592000' 'samesite=lax'; do
    pattern=";[[:space:]]*$attribute([[:space:]]*;|[[:space:]]*$)"
    [[ "$cookie_line" =~ $pattern ]] || fail "$request_label: $cookie_name missing $attribute"
  done
  pattern=';[[:space:]]*secure([=;[:space:]]|$)'
  [[ ! "$cookie_line" =~ $pattern ]] || fail "$request_label: local HTTP $cookie_name has Secure"
  case "$cookie_name" in
    clock_theme)
      pattern=';[[:space:]]*httponly([[:space:]]*;|[[:space:]]*$)'
      [[ "$cookie_line" =~ $pattern ]] || fail "$request_label: theme cookie missing HttpOnly"
      ;;
    clock_tz)
      pattern=';[[:space:]]*httponly([=;[:space:]]|$)'
      [[ ! "$cookie_line" =~ $pattern ]] || fail "$request_label: timezone cookie has HttpOnly"
      ;;
    *) fail "$request_label: unsupported preference cookie $cookie_name" ;;
  esac
}

reject_cookie() {
  reject_pattern "^set-cookie: $1=" "$headers" "unexpected $1 cookie write"
}

require_html() {
  require_status 200
  require_pattern '^content-type: text/html([;[:space:]]|$)' "$headers" 'missing HTML content type'
  require_pattern 'id="page-root" data-theme="' "$body" 'missing page root'
}

start_server() {
  local timeout=$1 listeners start_time now status
  listeners=$(ss -H -ltn '( sport = :5656 )') || fail 'could not check port 5656'
  [[ -z "$listeners" ]] || fail 'port 5656 is already in use; no process was stopped'

  setsid just dev 5656 >"$work_dir/server.log" 2>&1 &
  server_pid=$!
  tail --pid="$server_pid" --sleep-interval=0.2 -n +1 -f "$work_dir/server.log" >&2 &
  monitor_pid=$!
  start_time=$SECONDS
  while ((SECONDS - start_time < timeout)); do
    kill -0 "$server_pid" 2>/dev/null || fail 'just dev exited before healthy HTTP readiness'
    now=$(curl --disable --silent --show-error --noproxy '*' --proto '=http' \
      --connect-timeout 1 --max-time 2 --output "$work_dir/ready.body" \
      --write-out '%{http_code}' "$base_url/" 2>"$work_dir/ready.error")
    status=$?
    if ((status == 0)) && [[ "$now" == 200 ]]; then
      if rg --quiet 'id="page-root"' "$work_dir/ready.body"; then
        printf '%s\n' 'PASS: just dev 5656 serves a healthy page'
        return 0
      fi
    fi
    sleep 1
  done
  fail "no healthy HTTP page after ${timeout}s (last HTTP ${now:-none})"
}

theme_redirects() {
  request theme-dark POST '/theme?theme=dark'
  require_status 303
  require_pattern '^location: /[[:space:]]*$' "$headers" 'missing home redirect'
  require_cookie clock_theme dark

  request theme-invalid POST '/theme?theme=sepia'
  require_status 303
  require_pattern '^location: /[[:space:]]*$' "$headers" 'missing home redirect'
  require_cookie clock_theme light

  request theme-same POST '/theme?theme=dark' --header 'Cookie: clock_theme=dark'
  require_status 303
  require_pattern '^location: /[[:space:]]*$' "$headers" 'missing home redirect'
  reject_cookie clock_theme
  printf '%s\n' 'PASS: theme redirects and cookie suppression'
}

timezone_page() {
  local label=$1 zone=$2 path=$3 before after
  shift 3
  before=$(TZ="$zone" date +'%H.%M') || fail 'could not read expected local time'
  request "$label" GET "$path" "$@"
  after=$(TZ="$zone" date +'%H.%M') || fail 'could not read expected local time'
  require_html
  require_pattern "id=\"clock-time\" datetime=\"[^\"]+\">(${before//./\\.}|${after//./\\.})\\.[0-5][0-9]</time>" \
    "$body" "clock does not use $zone"
}

require_sse_frames() {
  local minimum=$1 maximum=$2 line state=0 count=0 first='' last='' current=''
  local clock_pattern='^data: <time id="clock-time" datetime="([01][0-9]|2[0-3]):([0-5][0-9]):([0-5][0-9])">([01][0-9]|2[0-3])\.([0-5][0-9])\.([0-5][0-9])</time>$'
  while IFS= read -r line || [[ -n "$line" ]]; do
    line=${line%$'\r'}
    case "$state:$line" in
      '0:event: clock') state=1 ;;
      1:data:*)
        [[ "$line" =~ $clock_pattern ]] || fail "$request_label: invalid clock data"
        [[ "${BASH_REMATCH[1]}:${BASH_REMATCH[2]}:${BASH_REMATCH[3]}" == \
          "${BASH_REMATCH[4]}:${BASH_REMATCH[5]}:${BASH_REMATCH[6]}" ]] ||
          fail "$request_label: datetime and clock display differ"
        current=${BASH_REMATCH[1]}:${BASH_REMATCH[2]}:${BASH_REMATCH[3]}
        state=2
        ;;
      '2:')
        ((count += 1))
        [[ -n "$first" ]] || first=$current
        last=$current
        state=0
        ;;
      '0:') ;;
      *) fail "$request_label: unexpected SSE frame content" ;;
    esac
  done <"$body"
  ((state == 0)) || fail "$request_label: incomplete SSE frame"
  ((count >= minimum && count <= maximum)) ||
    fail "$request_label: expected $minimum..$maximum complete events, received $count"
  if ((minimum > 1)); then
    [[ "$first" != "$last" ]] || fail "$request_label: streamed clock did not advance"
  fi
}

javascript_asset() {
  local asset_label=$1 asset_path=$2 redirect_path=${3:-}
  local location_header location expected_asset_hash actual_asset_hash
  request "$asset_label" GET "$asset_path"
  if [[ "$response_code" == 307 && -n "$redirect_path" ]]; then
    location_header=$(rg --ignore-case '^location: ' "$headers") ||
      fail "$request_label: missing asset redirect location"
    location=${location_header#*: }
    location=${location%$'\r'}
    [[ "$location" == "$redirect_path" ]] ||
      fail "$request_label: unexpected asset redirect"
    # Follow only this declared local path, never an arbitrary response URL.
    request "$asset_label-redirect" GET "$redirect_path"
  fi
  require_status 200
  require_pattern '^content-type: (text|application)/javascript([;[:space:]]|$)' \
    "$headers" 'missing JavaScript content type'
  [[ -s "$body" ]] || fail "$request_label: empty static JavaScript asset"
  expected_asset_hash=$(sha256sum "public${asset_path#/static}") ||
    fail "$request_label: could not hash the pinned asset"
  actual_asset_hash=$(sha256sum "$body") || fail "$request_label: could not hash the HTTP body"
  [[ "${actual_asset_hash%% *}" == "${expected_asset_hash%% *}" ]] ||
    fail "$request_label: HTTP asset differs from the pinned file"
}

worker_flows() {
  local stream_status before after
  request home GET /
  require_html
  require_pattern '^<!doctype html>' "$body" 'missing full HTML document'
  require_pattern 'id="page-root" data-theme="light"' "$body" 'wrong default theme'
  require_pattern 'hx-post="/theme\?theme=dark"' "$body" 'missing HTMX theme action'
  require_pattern 'sse-connect="/clock-stream"' "$body" 'missing SSE connection'

  request home-cookie GET / --header 'Cookie: clock_theme=dark'
  require_html
  require_pattern 'id="page-root" data-theme="dark"' "$body" 'theme cookie ignored'
  reject_cookie clock_theme

  request htmx-dark POST '/theme?theme=dark' --header 'HX-Request: true'
  require_html
  require_cookie clock_theme dark
  require_pattern 'id="page-root" data-theme="dark"' "$body" 'wrong HTMX theme'

  request htmx-same POST '/theme?theme=dark' --header 'HX-Request: true' --header 'Cookie: clock_theme=dark'
  require_html
  require_pattern 'id="page-root" data-theme="dark"' "$body" 'wrong HTMX theme'
  reject_cookie clock_theme

  request htmx-invalid POST '/theme?theme=sepia' --header 'HX-Request: true'
  require_html
  require_pattern 'id="page-root" data-theme="light"' "$body" 'invalid theme did not use light'
  require_cookie clock_theme light

  request theme-get GET '/theme?theme=dark'
  require_status 404
  require_pattern '^Not Found$' "$body" 'wrong theme GET body'

  request static GET /static/styles.css
  require_status 200
  require_pattern '^content-type: text/css([;[:space:]]|$)' "$headers" 'missing CSS content type'
  require_pattern 'clock-panel' "$body" 'missing stylesheet content'

  # Fetch the existing scripts as HTTP assets. Do not execute JavaScript.
  javascript_asset static-htmx /static/vendor/htmx.org@1.9.12/htmx.min.js \
    /vendor/htmx.org%401.9.12/htmx.min.js
  javascript_asset static-sse /static/vendor/htmx.org@1.9.12/dist/ext/sse.js \
    /vendor/htmx.org%401.9.12/dist/ext/sse.js
  javascript_asset static-timezone /static/timezone-bootstrap.js

  request static-missing GET /static/not-a-real-file.css
  require_status 404
  # ASSETS owns the missing-file body; its status is the stable contract.

  request route-missing GET /not-a-real-route
  require_status 404
  require_pattern '^Not Found$' "$body" 'wrong missing route body'

  timezone_page tz-trim Asia/Tokyo '/?tz=%20Asia%2FTokyo%20'
  require_cookie clock_tz 'Asia%2FTokyo'
  timezone_page tz-invalid UTC '/?tz=Mars%2FOlympus'
  reject_cookie clock_tz
  timezone_page tz-header-fallback Europe/Paris '/?tz=Mars%2FOlympus' --header 'x-timezone: Europe/Paris'
  require_cookie clock_tz 'Europe%2FParis'
  timezone_page tz-query-precedence America/Chicago '/?tz=America%2FChicago' \
    --header 'x-timezone: Europe/Paris' --header 'Cookie: clock_tz=Asia%2FTokyo'
  require_cookie clock_tz 'America%2FChicago'
  timezone_page tz-header-precedence Europe/Paris / \
    --header 'x-timezone: Europe/Paris' --header 'Cookie: clock_tz=Asia%2FTokyo'
  require_cookie clock_tz 'Europe%2FParis'
  timezone_page tz-cookie-fallback Asia/Tokyo / --header 'Cookie: clock_tz=Asia%2FTokyo'
  reject_cookie clock_tz
  timezone_page tz-invalid-fallback Asia/Tokyo '/?tz=Mars%2FOlympus' \
    --header 'x-timezone: Mars/Olympus' --header 'Cookie: clock_tz=Asia%2FTokyo'
  reject_cookie clock_tz
  timezone_page tz-same Asia/Tokyo '/?tz=Asia%2FTokyo' --header 'Cookie: clock_tz=Asia%2FTokyo'
  reject_cookie clock_tz
  timezone_page tz-header-same Europe/Paris / --header 'x-timezone: Europe/Paris' --header 'Cookie: clock_tz=Europe%2FParis'
  reject_cookie clock_tz

  request sse-once GET '/clock-stream?once=1'
  require_status 200
  require_pattern '^content-type: text/event-stream([;[:space:]]|$)' "$headers" 'missing SSE content type'
  require_pattern '^cache-control: .*no-cache' "$headers" 'missing SSE cache control'
  require_sse_frames 1 1

  before=$(TZ=Asia/Tokyo date +'%H.%M') || fail 'could not read expected Tokyo time'
  request sse-timezone-once GET '/clock-stream?once=1&tz=Asia%2FTokyo'
  after=$(TZ=Asia/Tokyo date +'%H.%M') || fail 'could not read expected Tokyo time'
  require_status 200
  require_pattern '^content-type: text/event-stream([;[:space:]]|$)' "$headers" 'missing SSE content type'
  require_cookie clock_tz 'Asia%2FTokyo'
  require_sse_frames 1 1
  require_pattern "^data: <time id=\"clock-time\" datetime=\"[^\"]+\">(${before//./\\.}|${after//./\\.})\\.[0-5][0-9]</time>" \
    "$body" 'SSE clock does not use Asia/Tokyo'

  request_label=sse-stream
  headers="$work_dir/$request_label.headers"
  body="$work_dir/$request_label.body"
  response_code=$(curl --disable --silent --show-error --noproxy '*' --proto '=http' \
    --connect-timeout 2 --max-time 4.5 --no-buffer --dump-header "$headers" \
    --output "$body" --write-out '%{http_code}' "$base_url/clock-stream" 2>"$work_dir/stream.error")
  stream_status=$?
  # A live stream must remain open until this client ends its bounded request.
  [[ "$stream_status" == 28 ]] || fail "sse-stream: expected client timeout, received curl exit $stream_status"
  require_status 200
  require_pattern '^content-type: text/event-stream([;[:space:]]|$)' "$headers" 'missing SSE content type'
  require_sse_frames 3 8
  request after-stream GET /
  require_html
  printf '%s\n' 'PASS: full page, HTMX, timezone precedence, cookies, assets, 404, and SSE'
}

main() {
  local mode=${1:-test-live} port=${2:-5656} argument=${3:-300} dependency script_dir
  (($# <= 3)) || fail 'usage: test-live.sh MODE [5656] [timeout-seconds]'
  [[ "$port" == 5656 ]] || fail 'live checks require explicit local port 5656'
  case "$mode" in
    test-live|verify-dev-start|verify-theme-redirect|verify-worker-flows) ;;
    verify-http-status)
      argument=${3:-200}
      [[ "$argument" =~ ^[1-5][0-9]{2}$ ]] || fail 'expected HTTP status must be 100..599'
      ;;
    *) fail "unknown live check: $mode" ;;
  esac
  if [[ "$mode" != verify-http-status ]]; then
    [[ "$argument" =~ ^[1-9][0-9]{0,3}$ ]] || fail 'timeout must be 1..3600 seconds'
    ((argument <= 3600)) || fail 'timeout must be 1..3600 seconds'
  fi
  for dependency in curl rg mktemp rm date sha256sum; do
    command -v "$dependency" >/dev/null 2>&1 || fail "missing command: $dependency"
  done
  case "$mode" in
    test-live|verify-dev-start)
      for dependency in just ss setsid tail sleep; do
        command -v "$dependency" >/dev/null 2>&1 || fail "missing command: $dependency"
      done
      ;;
  esac
  script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P) || fail 'could not locate scripts directory'
  cd -- "$script_dir/.." || fail 'could not enter project directory'
  work_dir=$(mktemp -d -t edge-clock-live.XXXXXXXX) || fail 'could not create test directory'
  trap cleanup EXIT
  trap 'exit 129' HUP
  trap 'exit 130' INT
  trap 'exit 143' TERM
  case "$mode" in
    test-live) start_server "$argument"; theme_redirects; worker_flows ;;
    verify-dev-start) start_server "$argument" ;;
    verify-theme-redirect) theme_redirects ;;
    verify-worker-flows) worker_flows ;;
    verify-http-status)
      request status GET /
      require_status "$argument"
      printf 'PASS: local home HTTP %s\n' "$argument"
      ;;
  esac
}

main "$@"
exit "$?"
