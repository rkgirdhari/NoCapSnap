#!/usr/bin/env bash
# Install or update the CapSnap server on the ZAP VPS (W3b). Run as root on the box,
# after preflight.sh passes. Safe to re-run: it changes only what differs.
#
#   sudo ./install.sh --domain feedback.example.com --binary ./capsnap-server
#
# Options:
#   --domain D          the guest-link domain (G1). DNS must already point at this box.
#   --binary PATH       the capsnap-server binary (x86_64, static musl build from CI).
#   --port N            loopback port for the server (default 8090).
#   --tls-cert PATH     use an existing certificate (full chain) instead of Let's Encrypt,
#   --tls-key PATH      with its key.
#   --email ADDR        Let's Encrypt account email, if certbot has no account yet.
#
# What it does:
# - Creates the system user `capsnap`, and puts the binary in /var/www/capsnap/releases/.
# - Writes /etc/capsnap/capsnap.env; the data lives in /var/lib/capsnap.
# - Installs the systemd unit, a journald namespace with 30-day retention, nginx
#   `server` blocks for the domain, and the capsnapctl and capsnap-release tools.
# - Gets a certificate with certbot (webroot), unless one is given.
#
# nginx is only reloaded after `nginx -t` passes. If our block breaks the test, the
# previous version is put back, so the other sites on the box keep working.
set -euo pipefail

HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=/var/www/capsnap
ETC=/etc/capsnap
ACME_ROOT=/var/www/letsencrypt

DOMAIN=
BINARY=
PORT=8090
TLS_CERT=
TLS_KEY=
EMAIL=

say() { printf '==> %s\n' "$*"; }
die() { printf 'install.sh: %s\n' "$*" >&2; exit 1; }

while [ $# -gt 0 ]; do
    case "$1" in
        --domain) DOMAIN=${2:-}; shift 2 ;;
        --binary) BINARY=${2:-}; shift 2 ;;
        --port) PORT=${2:-}; shift 2 ;;
        --tls-cert) TLS_CERT=${2:-}; shift 2 ;;
        --tls-key) TLS_KEY=${2:-}; shift 2 ;;
        --email) EMAIL=${2:-}; shift 2 ;;
        -h | --help) sed -n '2,24p' "$0"; exit 0 ;;
        *) die "unknown option: $1 (see --help)" ;;
    esac
done

# --- checks -------------------------------------------------------------------------

[ "$(id -u)" -eq 0 ] || die "run as root"
[[ $DOMAIN =~ ^([a-z0-9]([a-z0-9-]*[a-z0-9])?\.)+[a-z]{2,}$ ]] ||
    die "--domain must be a lowercase host name such as feedback.example.com"
[ -f "$BINARY" ] || die "--binary must point at the capsnap-server binary"
if ! [[ $PORT =~ ^[0-9]+$ ]] || [ "$PORT" -lt 1024 ] || [ "$PORT" -gt 65535 ]; then
    die "--port must be 1024-65535"
fi
if [ -n "$TLS_CERT$TLS_KEY" ]; then
    if [ ! -f "$TLS_CERT" ] || [ ! -f "$TLS_KEY" ]; then
        die "--tls-cert and --tls-key must both be existing files"
    fi
fi
for tool in systemctl nginx curl runuser sha256sum; do
    command -v "$tool" >/dev/null || die "$tool is not installed"
done
systemd_version=$(systemctl --version | awk 'NR == 1 { print $2 }')
[ "${systemd_version%%[!0-9]*}" -ge 245 ] ||
    die "systemd $systemd_version is too old for LogNamespace (needs 245 or later)"
version=$(chmod +x "$BINARY" && "$BINARY" version 2>&1) ||
    die "the binary does not run on this box: $version"
[[ $version == capsnap-server\ * ]] || die "not a capsnap-server binary: $version"
UPSTREAM="127.0.0.1:$PORT"

# --- user, directories, settings ----------------------------------------------------

if ! id capsnap >/dev/null 2>&1; then
    say "creating the system user capsnap"
    useradd --system --user-group --home-dir /var/lib/capsnap --no-create-home \
        --shell /usr/sbin/nologin capsnap
fi
install -d -m 0755 "$ROOT" "$ROOT/releases"
install -d -m 0750 -g capsnap "$ETC"

env_file="$ETC/capsnap.env"
if [ -f "$env_file" ]; then
    current_base=$(sed -n 's/^CAPSNAP_PUBLIC_BASE_URL=//p' "$env_file" | tail -n 1)
    [ "$current_base" = "https://$DOMAIN" ] ||
        die "$env_file points guest links at $current_base, not https://$DOMAIN.
Changing it breaks every guest QR already shown. If that is intended, edit the file yourself and re-run."
    current_bind=$(sed -n 's/^CAPSNAP_BIND=//p' "$env_file" | tail -n 1)
    [ "$current_bind" = "$UPSTREAM" ] ||
        die "$env_file binds $current_bind, not $UPSTREAM; pass --port ${current_bind##*:} or edit the file"
else
    say "writing $env_file"
    sed -e "s|@DOMAIN@|$DOMAIN|g" -e "s|@UPSTREAM@|$UPSTREAM|g" "$HERE/capsnap.env.in" >"$env_file.new"
    chown root:capsnap "$env_file.new"
    chmod 0640 "$env_file.new"
    mv -f "$env_file.new" "$env_file"
fi

# --- release ------------------------------------------------------------------------

sum=$(sha256sum "$BINARY" | cut -c1-12)
current_sum=
if [ -x "$ROOT/current/capsnap-server" ]; then
    current_sum=$(sha256sum "$ROOT/current/capsnap-server" | cut -c1-12)
fi
release=
if [ "$sum" != "$current_sum" ]; then
    release="$ROOT/releases/$(date -u +%Y%m%d%H%M%S)-$sum"
    say "staging release $(basename "$release") ($version)"
    install -D -m 0755 "$BINARY" "$release/capsnap-server"
fi

# --- systemd, journald, tools -------------------------------------------------------

say "installing the systemd unit, journald namespace and tools"
install -m 0644 "$HERE/systemd/capsnap.service" /etc/systemd/system/capsnap.service
install -m 0644 "$HERE/systemd/journald@capsnap.conf" /etc/systemd/journald@capsnap.conf
install -m 0755 "$HERE/bin/capsnap-release" /usr/local/sbin/capsnap-release
install -m 0755 "$HERE/bin/capsnapctl" /usr/local/sbin/capsnapctl
systemctl daemon-reload
systemctl enable capsnap.service >/dev/null

# --- nginx --------------------------------------------------------------------------

if [ -d /etc/nginx/sites-available ] && [ -d /etc/nginx/sites-enabled ]; then
    site=/etc/nginx/sites-available/capsnap.conf
    link=/etc/nginx/sites-enabled/capsnap.conf
else
    site=/etc/nginx/conf.d/capsnap.conf
    link=
fi
has_ipv6() { [ -s /proc/net/if_inet6 ]; }

# render <with-tls: yes|no> <cert> <key>
render() {
    local out
    out=$(sed -e "s|@DOMAIN@|$DOMAIN|g" -e "s|@UPSTREAM@|$UPSTREAM|g" \
        -e "s|@CERT@|$2|g" -e "s|@KEY@|$3|g" "$HERE/nginx/capsnap.conf.in")
    if [ "$1" = no ]; then
        out=$(printf '%s\n' "$out" | sed '/^# BEGIN tls$/,/^# END tls$/d')
    fi
    if ! has_ipv6; then
        out=$(printf '%s\n' "$out" | sed '/# ipv6$/d')
    fi
    printf '%s\n' "$out"
}

# apply_site <content>: write our site, test the whole nginx config, reload, or put back.
apply_site() {
    local backup=
    if [ -f "$site" ]; then
        backup=$(mktemp)
        cp -p "$site" "$backup"
    fi
    printf '%s\n' "$1" >"$site.new"
    chmod 0644 "$site.new"
    mv -f "$site.new" "$site"
    [ -z "$link" ] || ln -sfn "$site" "$link"
    local log
    log=$(mktemp)
    if ! nginx -t 2>"$log"; then
        cat "$log" >&2
        rm -f "$log"
        if [ -n "$backup" ]; then
            mv -f "$backup" "$site"
        else
            rm -f "$site"
            [ -z "$link" ] || rm -f "$link"
        fi
        die "nginx rejected the CapSnap block; the previous config is back in place, nginx was not reloaded"
    fi
    rm -f "$log"
    [ -z "$backup" ] || rm -f "$backup"
    systemctl reload nginx
}

if [ -n "$TLS_CERT" ]; then
    cert=$(readlink -f "$TLS_CERT")
    key=$(readlink -f "$TLS_KEY")
else
    cert=/etc/letsencrypt/live/$DOMAIN/fullchain.pem
    key=/etc/letsencrypt/live/$DOMAIN/privkey.pem
    if [ ! -f "$cert" ]; then
        command -v certbot >/dev/null ||
            die "no certificate for $DOMAIN and certbot is not installed (apt install certbot), or pass --tls-cert/--tls-key"
        say "serving the ACME challenge for $DOMAIN over http"
        install -d -m 0755 "$ACME_ROOT"
        apply_site "$(render no - -)"
        say "requesting a Let's Encrypt certificate for $DOMAIN"
        email_args=()
        if [ -n "$EMAIL" ]; then email_args=(--email "$EMAIL"); fi
        certbot certonly --webroot -w "$ACME_ROOT" -d "$DOMAIN" \
            --non-interactive --agree-tos --keep-until-expiring "${email_args[@]}" \
            --deploy-hook "systemctl reload nginx"
    fi
fi
say "writing the nginx server blocks for $DOMAIN ($site)"
apply_site "$(render yes "$cert" "$key")"

# --- start --------------------------------------------------------------------------

if [ -n "$release" ]; then
    capsnap-release activate "$release"
else
    say "the binary is unchanged; restarting to pick up any unit or settings change"
    systemctl restart capsnap
    capsnap-release status >/dev/null || die "the server is not healthy; see journalctl -u capsnap and journalctl --namespace=capsnap"
fi

say "checking https://$DOMAIN through nginx"
curl -fsS --noproxy '*' --max-time 10 --resolve "$DOMAIN:443:127.0.0.1" "https://$DOMAIN/api/v1/health" |
    grep -q '"status":"ok"' ||
    die "the server is up but https://$DOMAIN/api/v1/health failed through nginx"

cat <<EOF

CapSnap is running at https://$DOMAIN ($version, listening on $UPSTREAM).

Next:
  1. From your own computer:  ./smoke.sh $DOMAIN
  2. Create the first restaurant and admin (test data only until the W4 restore drill):
       capsnapctl create-org <slug> "<Name>"
       capsnapctl create-location <slug> "<Location name>" <IANA timezone, e.g. Europe/Berlin>
       read -rs PW && printf '%s\n' "\$PW" | capsnapctl create-staff <slug> <login> "<Display name>" admin <location-id>
  3. Logs: journalctl --namespace=capsnap (the server), journalctl -u capsnap (start and stop)
EOF
