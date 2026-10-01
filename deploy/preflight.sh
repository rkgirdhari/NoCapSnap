#!/usr/bin/env bash
# W3b step 1: read-only checks on the ZAP VPS before anything is installed.
#
#   sudo ./preflight.sh [feedback.example.com]
#
# It changes nothing. The output has no passwords, keys or IP addresses, so it is safe
# to send back. The domain argument (G1) is optional; with it, DNS is checked too.
# Exits non-zero if any check FAILs; WARN lines need a look but don't block.
set -uo pipefail

DOMAIN=${1:-}
PORT=${CAPSNAP_PORT:-8090}
fails=0

ok() { printf 'OK    %s\n' "$*"; }
warn() { printf 'WARN  %s\n' "$*"; }
bad() { printf 'FAIL  %s\n' "$*"; fails=$((fails + 1)); }
info() { printf 'INFO  %s\n' "$*"; }
have() { command -v "$1" >/dev/null 2>&1; }

echo "CapSnap preflight ($(date -u +%Y-%m-%dT%H:%M:%SZ))"
echo

# The machine
arch=$(uname -m)
info "kernel: $(uname -sr), architecture $arch"
if [ "$arch" = x86_64 ]; then ok "architecture x86_64 (the CI build targets x86_64-unknown-linux-musl)"
else bad "architecture $arch: the CI build is x86_64 only; W3b needs an aarch64 build for this box"; fi
if [ -r /etc/os-release ]; then
    # shellcheck disable=SC1091
    . /etc/os-release
    info "OS: ${PRETTY_NAME:-unknown}"
fi
info "CPUs: $(nproc)"
mem_mb=$(awk '/^MemTotal:/ { print int($2 / 1024) }' /proc/meminfo)
avail_mb=$(awk '/^MemAvailable:/ { print int($2 / 1024) }' /proc/meminfo)
info "memory: ${mem_mb} MB total, ${avail_mb} MB available"
if [ "$avail_mb" -ge 256 ]; then ok "memory: CapSnap needs well under 100 MB"
else warn "memory: only ${avail_mb} MB available"; fi
for dir in / /var/lib /var/www; do
    [ -d "$dir" ] || continue
    free_gb=$(df -Pk "$dir" | awk 'NR == 2 { printf "%.1f", $4 / 1048576 }')
    info "disk: $free_gb GB free on the filesystem holding $dir"
done
free_var_kb=$(df -Pk /var/lib | awk 'NR == 2 { print $4 }')
if [ "$free_var_kb" -ge $((5 * 1048576)) ]; then ok "disk: at least 5 GB free for photos (kept 30 days after sync)"
else warn "disk: less than 5 GB free under /var/lib for photos"; fi

# systemd and journald
if have systemctl && [ -d /run/systemd/system ]; then
    sv=$(systemctl --version | awk 'NR == 1 { print $2 }')
    sv=${sv%%[!0-9]*}
    if [ "$sv" -ge 245 ]; then ok "systemd $sv (journal namespaces need 245 or later)"
    else bad "systemd $sv: too old for LogNamespace (needs 245 or later)"; fi
    info "journald retention for everything else: $(grep -hE '^\s*MaxRetentionSec' /etc/systemd/journald.conf /etc/systemd/journald.conf.d/*.conf 2>/dev/null | tail -n 1 || true)"
else
    bad "systemd is not running as the init system"
fi

# nginx and TLS
if have nginx; then
    ok "nginx: $(nginx -v 2>&1 | sed 's/^nginx version: //')"
    if [ "$(id -u)" -eq 0 ]; then
        if nginx -t >/dev/null 2>&1; then ok "nginx: the current configuration tests clean"
        else bad "nginx: 'nginx -t' fails already, before CapSnap; fix that first"; fi
    else
        warn "nginx: run with sudo to test the current configuration"
    fi
    if [ -d /etc/nginx/sites-enabled ]; then info "nginx: sites-available/sites-enabled layout"
    else info "nginx: conf.d layout"; fi
    if grep -rqs 'sites-enabled\|conf\.d' /etc/nginx/nginx.conf; then ok "nginx: nginx.conf includes site files"
    else warn "nginx: nginx.conf includes neither sites-enabled nor conf.d; the CapSnap block would not load"; fi
    if [ -e /etc/nginx/sites-enabled/capsnap.conf ] || [ -e /etc/nginx/conf.d/capsnap.conf ]; then
        info "nginx: a CapSnap block already exists (a re-run updates it)"
    fi
else
    bad "nginx is not installed (the ZAP docs say it fronts ports 80 and 443)"
fi
if have certbot; then ok "certbot: $(certbot --version 2>&1 | tail -n 1)"
else warn "certbot is not installed; install it (apt install certbot) or pass --tls-cert/--tls-key to install.sh"; fi

# Ports
if have ss; then
    listeners=$(ss -Hltn 2>/dev/null | awk '{ print $4 }')
    if printf '%s\n' "$listeners" | grep -Eq "[:.]$PORT\$"; then bad "port $PORT is already in use; choose another with install.sh --port"
    else ok "port $PORT is free for the server (loopback only)"; fi
    for p in 80 443; do
        if printf '%s\n' "$listeners" | grep -Eq "[:.]$p\$"; then ok "port $p: something is listening (nginx expected)"
        else warn "port $p: nothing is listening"; fi
    done
else
    warn "ss is not available; cannot check ports"
fi
if [ -s /proc/net/if_inet6 ]; then info "IPv6: available (nginx will also listen on [::])"
else info "IPv6: not available (IPv4 only)"; fi
if have ufw && [ "$(id -u)" -eq 0 ]; then
    info "firewall: $(ufw status 2>/dev/null | head -n 1)"
fi

# Tools install.sh uses
for tool in curl runuser sha256sum useradd openssl; do
    if have "$tool"; then ok "tool: $tool"; else bad "tool: $tool is missing"; fi
done

# Earlier installs
if id capsnap >/dev/null 2>&1; then info "user capsnap exists (a re-run keeps it)"; fi
if [ -d /var/lib/capsnap ]; then info "data directory /var/lib/capsnap exists: $(du -sh /var/lib/capsnap 2>/dev/null | cut -f1)"; fi

# DNS for the guest domain (G1)
if [ -n "$DOMAIN" ]; then
    resolved=$(getent ahostsv4 "$DOMAIN" 2>/dev/null | awk '{ print $1 }' | sort -u)
    if [ -z "$resolved" ]; then
        bad "DNS: $DOMAIN does not resolve yet; add an A record pointing at this box"
    else
        local_ips=$(hostname -I 2>/dev/null | tr ' ' '\n')
        match=no
        for ip in $resolved; do
            if printf '%s\n' "$local_ips" | grep -qx "$ip"; then match=yes; fi
        done
        if [ "$match" = yes ]; then ok "DNS: $DOMAIN points at this box"
        else warn "DNS: $DOMAIN resolves, but not to an address on this box (fine behind NAT; otherwise fix the A record)"; fi
    fi
else
    info "DNS: no domain given (G1 still open); re-run as 'preflight.sh <domain>' once it is chosen"
fi

echo
if [ "$fails" -eq 0 ]; then echo "Preflight passed. Send this output back, then run install.sh."
else echo "Preflight found $fails blocking problem(s)."; fi
[ "$fails" -eq 0 ]
