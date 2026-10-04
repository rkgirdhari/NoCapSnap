#!/bin/bash
# Make a non-root user on the VPS for working with Claude Code. Run as root ON THE BOX, normally through
# `owner-setup.ps1 -Step ClaudeUser`, which passes your PUBLIC key as the only argument:
#
#   bash box-claude-user.sh "ssh-ed25519 AAAA… comment" [username]
#
# What it does: creates the user (no password, key login only), installs git, tmux, curl, unzip and
# Node.js from Ubuntu's own packages, and installs Claude Code under the user's home (no sudo).
# What it does not do: it does not touch sshd, ufw or nginx; it does not give the user sudo (root
# stays with your key, for the CapSnap deploy kit); and it is safe to run again.
set -euo pipefail

PUBKEY=${1:-}
NAME=${2:-hcc}
[ "$(id -u)" -eq 0 ] || { echo "run as root" >&2; exit 1; }
[[ $NAME =~ ^[a-z][a-z0-9_-]{1,30}$ ]] || { echo "bad user name: $NAME" >&2; exit 1; }
[[ $PUBKEY =~ ^ssh-ed25519\ [A-Za-z0-9+/=]+(\ [A-Za-z0-9@._-]+)?$ ]] || { echo "pass one ed25519 PUBLIC key line as the first argument" >&2; exit 1; }

export DEBIAN_FRONTEND=noninteractive
apt-get update -q
apt-get install -y -q git tmux curl unzip nodejs npm

node_major=$(node -p 'process.versions.node.split(".")[0]')
[ "$node_major" -ge 20 ] || { echo "Ubuntu's nodejs is $node_major.x; Claude Code needs 20 or newer" >&2; exit 1; }

if id "$NAME" >/dev/null 2>&1; then
    echo "user $NAME exists; keeping it"
else
    adduser --disabled-password --gecos "" "$NAME"
fi
install -d -m 700 -o "$NAME" -g "$NAME" "/home/$NAME/.ssh"
touch "/home/$NAME/.ssh/authorized_keys"
if ! grep -qxF "$PUBKEY" "/home/$NAME/.ssh/authorized_keys"; then
    echo "$PUBKEY" >> "/home/$NAME/.ssh/authorized_keys"
fi
chown "$NAME:$NAME" "/home/$NAME/.ssh/authorized_keys"
chmod 600 "/home/$NAME/.ssh/authorized_keys"

# Claude Code, installed into the user's own prefix so no sudo is needed.
su - "$NAME" -c '
    set -e
    mkdir -p ~/.npm-global
    npm config set prefix ~/.npm-global
    grep -q ".npm-global/bin" ~/.profile || echo "export PATH=\$HOME/.npm-global/bin:\$PATH" >> ~/.profile
    npm install -g @anthropic-ai/claude-code
    ~/.npm-global/bin/claude --version
'
echo "done: ssh $NAME@$(hostname -I | awk '{print $1}'), then: tmux new -s work, then: claude"
