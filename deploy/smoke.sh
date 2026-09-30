#!/usr/bin/env bash
# Check a CapSnap install from the outside (W3b). Run it from your own computer, not
# on the box, so the "not reachable" checks mean something:
#
#   ./smoke.sh feedback.example.com
#
# Needs curl and openssl. Changes nothing on the server. Exits non-zero if any check fails.
# SMOKE_PORT is the server's loopback port (default 8090), which must NOT answer from outside.
set -uo pipefail

DOMAIN=${1:-}
PORT=${SMOKE_PORT:-8090}
[ -n "$DOMAIN" ] || { echo "usage: $0 <domain>" >&2; exit 2; }
BASE="https://$DOMAIN"
failures=0
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

pass() { printf 'PASS  %s\n' "$*"; }
fail() { printf 'FAIL  %s\n' "$*"; failures=$((failures + 1)); }

# status <url> [curl args...]: print the HTTP status; headers to $tmp/h, body to $tmp/b.
status() {
    local url=$1
    shift
    curl -sS --max-time 15 -o "$tmp/b" -D "$tmp/h" -w '%{http_code}' "$@" "$url" 2>"$tmp/err" || true
}
header() { grep -i "^$1:" "$tmp/h" | tail -n 1 | cut -d: -f2- | tr -d '\r' | sed 's/^ *//'; }

# 1. Health over https, with the certificate verified.
code=$(status "$BASE/api/v1/health")
if [ "$code" = 200 ] && grep -q '"status":"ok"' "$tmp/b"; then
    pass "health: $BASE/api/v1/health answers ok over verified TLS"
else
    fail "health: got '$code' $(head -c 200 "$tmp/err" "$tmp/b" 2>/dev/null | tr '\n' ' ')"
fi

# 2. Certificate: verifies for the name, and has at least 14 days left.
cert=$(echo | openssl s_client -connect "$DOMAIN:443" -servername "$DOMAIN" -verify_hostname "$DOMAIN" \
    -verify_return_error 2>/dev/null | openssl x509 2>/dev/null)
if [ -z "$cert" ]; then
    fail "certificate: openssl could not verify a certificate for $DOMAIN"
elif printf '%s\n' "$cert" | openssl x509 -noout -checkend $((14 * 86400)) >/dev/null; then
    pass "certificate: valid for $DOMAIN until $(printf '%s\n' "$cert" | openssl x509 -noout -enddate | cut -d= -f2)"
else
    fail "certificate: expires within 14 days"
fi

# 3. Plain http redirects to https.
code=$(status "http://$DOMAIN/g/")
location=$(header location)
if [ "$code" = 301 ] && [ "$location" = "$BASE/g/" ]; then
    pass "http: redirects to $location"
else
    fail "http: expected 301 to $BASE/g/, got '$code' to '$location'"
fi

# 4. The guest page: served, with HSTS and a strict CSP, and no server version.
code=$(status "$BASE/g/")
csp=$(header content-security-policy)
hsts=$(header strict-transport-security)
server=$(header server)
if [ "$code" = 200 ] && grep -q '<title>Private feedback</title>' "$tmp/b"; then
    pass "guest page: $BASE/g/ is the CapSnap portal"
else
    fail "guest page: got '$code'"
fi
case "$csp" in
    *"default-src 'none'"*) pass "guest page: CSP starts from default-src 'none'" ;;
    *) fail "guest page: CSP is '$csp'" ;;
esac
case "$hsts" in
    max-age=*) pass "HSTS: $hsts" ;;
    *) fail "HSTS: missing" ;;
esac
case "$server" in
    *[0-9]*) fail "server header reveals a version: $server" ;;
    *) pass "server header: '${server:-none}' (no version)" ;;
esac

# 5. Photo uploads get through nginx: 2 MiB reaches the server (401 without a session),
#    11 MiB is stopped at the edge (413). nginx's own default would refuse both.
head -c $((2 * 1024 * 1024)) /dev/zero >"$tmp/2m"
head -c $((11 * 1024 * 1024)) /dev/zero >"$tmp/11m"
code=$(status "$BASE/api/v1/media" -X POST -H 'content-type: image/jpeg' --data-binary @"$tmp/2m")
if [ "$code" = 401 ]; then pass "upload: a 2 MiB photo reaches the server (401 without a session)"
else fail "upload: a 2 MiB body got '$code' (nginx client_max_body_size?)"; fi
code=$(status "$BASE/api/v1/media" -X POST -H 'content-type: image/jpeg' --data-binary @"$tmp/11m")
if [ "$code" = 413 ]; then pass "upload: 11 MiB is refused (413)"
else fail "upload: an 11 MiB body got '$code'"; fi

# 6. A made-up guest token is refused cleanly, not with a server error.
code=$(status "$BASE/api/v1/guest/session" -X POST -H 'content-type: application/json' \
    --data '{"token":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"}')
if [[ $code =~ ^4[0-9][0-9]$ ]]; then pass "guest: an unknown token is refused ($code)"
else fail "guest: an unknown token got '$code'"; fi

# 7. The server's own port is not reachable from outside; only nginx is.
if curl -sS --max-time 5 -o /dev/null "http://$DOMAIN:$PORT/api/v1/health" 2>/dev/null; then
    fail "exposure: port $PORT answers from outside; only 80 and 443 should"
else
    pass "exposure: port $PORT does not answer from outside"
fi

echo
if [ "$failures" -eq 0 ]; then echo "All checks passed for $DOMAIN."; else echo "$failures check(s) failed for $DOMAIN."; fi
[ "$failures" -eq 0 ]
