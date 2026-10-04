<#
.SYNOPSIS
  The owner's remaining setup steps for CapSnap, from your Windows PC (see To-Do.md).

.DESCRIPTION
  Run one step at a time:   .\scripts\owner-setup.ps1 -Step <name> [options]

  Check        which tools are installed (ssh, scp, gh, git bash, age, keytool)
  Dns          what the guest domain and the company domain resolve to (W3b step 2)
  SshKey       make or load your SSH key; -InstallKey copies the public half to the box (W3b step 3)
  BackupKey    make the age key pair for encrypted backups; the PRIVATE key stays on this PC (W4)
  UploadKey    make the Play upload key and, with -SetSecrets, the four GitHub secrets (W5)
  Binary       download and verify the latest capsnap-server build from CI
  BoxPrep      packages, firewall, key-only logins on a freshly installed box (asks before each change)
  Deploy       copy the deploy kit, run preflight, then (after you confirm) install
  Smoke        run smoke.sh against the live domain

  What it never does: it never reinstalls or wipes the VPS (that is the ZAP panel, your step),
  never changes DNS, and never reads or types a password for you. ssh-keygen, ssh, keytool and
  age-keygen ask for passphrases and passwords themselves. Every change on the box asks first.

.EXAMPLE
  .\scripts\owner-setup.ps1 -Step Check
  .\scripts\owner-setup.ps1 -Step SshKey -InstallKey -Box 5.249.163.79
  .\scripts\owner-setup.ps1 -Step Deploy -Box 5.249.163.79
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [ValidateSet('Check', 'Dns', 'SshKey', 'BackupKey', 'UploadKey', 'Binary', 'BoxPrep', 'Deploy', 'Smoke')]
    [string] $Step,

    [string] $Box = '5.249.163.79',
    [string] $User = 'root',
    [string] $Domain = 'nocapsnap.hammurabi.click',
    [string] $Repo = 'rkgirdhari/NoCapSnap',

    [switch] $InstallKey,   # SshKey: copy the public key to the box (ssh asks for the root password once)
    [switch] $PushRecipients, # BackupKey: put the PUBLIC key on the box as /etc/capsnap/backup.recipients
    [switch] $SetSecrets    # UploadKey: store the four GitHub secrets with gh
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$sshDir = Join-Path $env:USERPROFILE '.ssh'
$keyPath = Join-Path $sshDir 'id_ed25519'
$backupKey = Join-Path $env:USERPROFILE 'capsnap-backup-key.txt'
$target = "$User@$Box"

function Say($m) { Write-Host "==> $m" -ForegroundColor Cyan }
function Warn($m) { Write-Host "!!  $m" -ForegroundColor Yellow }
function Need($tool, $hint) {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) { throw "$tool is not installed. $hint" }
}
function Confirm-Step($question) {
    $a = Read-Host "$question [y/N]"
    return $a -match '^(y|yes)$'
}
# Run a command on the box. Output is shown; a non-zero exit stops the script.
function Remote([string] $cmd) {
    & ssh -o BatchMode=yes $target $cmd
    if ($LASTEXITCODE -ne 0) { throw "on the box: '$cmd' failed (exit $LASTEXITCODE)" }
}
function Test-KeyLogin {
    & ssh -o BatchMode=yes -o ConnectTimeout=10 $target 'true' 2>$null
    return ($LASTEXITCODE -eq 0)
}

switch ($Step) {

    'Check' {
        $tools = [ordered]@{
            ssh        = 'Windows optional feature "OpenSSH Client"'
            scp        = 'comes with OpenSSH Client'
            'ssh-keygen' = 'comes with OpenSSH Client'
            gh         = 'winget install GitHub.cli, then gh auth login'
            git        = 'winget install Git.Git (also gives you bash, needed for smoke.sh)'
            age        = 'winget install FiloSottile.age'
            keytool    = 'comes with a JDK (winget install EclipseAdoptium.Temurin.21.JDK)'
        }
        foreach ($t in $tools.Keys) {
            if (Get-Command $t -ErrorAction SilentlyContinue) { Write-Host ("  ok       {0}" -f $t) }
            else { Write-Host ("  MISSING  {0}   {1}" -f $t, $tools[$t]) -ForegroundColor Yellow }
        }
        Say "SSH key: $(if (Test-Path $keyPath) { 'found' } else { 'none yet (run -Step SshKey)' })"
        Say "Backup key: $(if (Test-Path $backupKey) { 'found' } else { 'none yet (run -Step BackupKey)' })"
        Say 'Box login with your key:'
        if (Get-Command ssh -ErrorAction SilentlyContinue) {
            if (Test-KeyLogin) { Write-Host '  ok       key login works' }
            else { Warn "key login to $target does not work yet (key not on the box, or a passphrase key that is not loaded in ssh-agent)" }
        }
    }

    'Dns' {
        foreach ($name in @($Domain, 'hcc.software', 'www.hcc.software', 'hammurabi.click')) {
            $a = Resolve-DnsName -Name $name -Type A -ErrorAction SilentlyContinue | Where-Object { $_.Type -eq 'A' }
            $ips = ($a | ForEach-Object { $_.IPAddress }) -join ', '
            Write-Host ("  {0,-28} {1}" -f $name, $(if ($ips) { $ips } else { '(no A record)' }))
        }
        Say "Goal for ${Domain}: exactly one A record, $Box. More than one means some visitors reach the other address."
        Say 'Change DNS where the domain is managed; this script does not touch DNS.'
    }

    'SshKey' {
        Need ssh-keygen 'Add the Windows optional feature "OpenSSH Client".'
        if (-not (Test-Path $keyPath)) {
            Say 'Making an ed25519 key. Choose a passphrase when ssh-keygen asks.'
            New-Item -ItemType Directory -Force $sshDir | Out-Null
            & ssh-keygen -t ed25519 -f $keyPath -C "capsnap-owner"
            if ($LASTEXITCODE -ne 0) { throw 'ssh-keygen failed' }
        } else { Say "Using the existing key $keyPath" }

        $agent = Get-Service ssh-agent -ErrorAction SilentlyContinue
        if ($agent -and $agent.Status -ne 'Running') {
            try { Start-Service ssh-agent } catch { Warn 'Could not start ssh-agent (needs an administrator PowerShell once: Set-Service ssh-agent -StartupType Automatic; Start-Service ssh-agent).' }
        }
        Say 'Loading the key into ssh-agent (asks for the passphrase):'
        & ssh-add $keyPath

        Say 'Your PUBLIC key (safe to share; this is what goes in the ZAP panel or on the box):'
        Get-Content "$keyPath.pub"

        if ($InstallKey) {
            Say "Copying the public key to $target. ssh will ask for the root password once."
            $pub = (Get-Content "$keyPath.pub" -Raw).Trim()
            $pub | & ssh $target "umask 077; mkdir -p ~/.ssh; cat >> ~/.ssh/authorized_keys"
            if ($LASTEXITCODE -ne 0) { throw 'could not install the key' }
            if (Test-KeyLogin) { Say 'Key login works.' } else { Warn 'Key login still fails; check the passphrase is loaded (ssh-add).' }
        } else {
            Say 'To put it on the box now, re-run with -InstallKey (or paste it into the ZAP panel at reinstall).'
        }
        Warn 'After a reinstall ssh may warn that the host key changed. That is expected once: ssh-keygen -R <box>, then connect again.'
    }

    'BackupKey' {
        Need age-keygen 'winget install FiloSottile.age'
        if (Test-Path $backupKey) { throw "$backupKey already exists. Not overwriting it: losing this file makes every backup unreadable." }
        & age-keygen -o $backupKey
        if ($LASTEXITCODE -ne 0) { throw 'age-keygen failed' }
        $public = (& age-keygen -y $backupKey).Trim()
        Say 'Public key (goes on the box):'
        Write-Host "  $public"
        Warn "PRIVATE key saved to $backupKey. Copy it to TWO places OFF this PC (password manager entry, USB stick). Never put it on the box or in the repository."
        if ($PushRecipients) {
            if (-not (Test-KeyLogin)) { throw "key login to $target does not work yet; run -Step SshKey -InstallKey first" }
            if (Confirm-Step "Write the PUBLIC key to /etc/capsnap/backup.recipients on $Box?") {
                # The capsnap group exists only after install.sh; create the file root-only and let install.sh re-run fix group access.
                $public | & ssh -o BatchMode=yes $target 'install -d -m 0750 /etc/capsnap; cat > /etc/capsnap/backup.recipients; chmod 0640 /etc/capsnap/backup.recipients; getent group capsnap >/dev/null && chgrp capsnap /etc/capsnap/backup.recipients; true'
                Say 'Done. Re-run install.sh (-Step Deploy) so the backup timer is enabled.'
            }
        }
    }

    'UploadKey' {
        Need keytool 'Install a JDK (winget install EclipseAdoptium.Temurin.21.JDK).'
        $jks = Join-Path $env:USERPROFILE 'capsnap-upload.jks'
        if (Test-Path $jks) { Say "Using the existing $jks" }
        else {
            Say 'Making the Play upload key. keytool asks for the keystore password and your details.'
            & keytool -genkeypair -v -keystore $jks -storetype PKCS12 -alias upload -keyalg RSA -keysize 4096 -validity 9125
            if ($LASTEXITCODE -ne 0) { throw 'keytool failed' }
        }
        Warn "Back up $jks and its password somewhere safe. If the key is lost, Google has to reset it."
        if ($SetSecrets) {
            Need gh 'winget install GitHub.cli, then gh auth login'
            $sec = Read-Host 'Keystore password (also used as the key password for PKCS12)' -AsSecureString
            $bstr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($sec)
            try {
                $pw = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($bstr)
                if (-not (Confirm-Step "Store 4 secrets (keystore, alias, 2 passwords) in $Repo ?")) { break }
                # Values go over stdin, never on a command line.
                [Convert]::ToBase64String([IO.File]::ReadAllBytes($jks)) | & gh secret set ANDROID_UPLOAD_KEYSTORE_BASE64 --repo $Repo
                'upload' | & gh secret set ANDROID_UPLOAD_KEY_ALIAS --repo $Repo
                $pw | & gh secret set ANDROID_UPLOAD_KEYSTORE_PASSWORD --repo $Repo
                $pw | & gh secret set ANDROID_UPLOAD_KEY_PASSWORD --repo $Repo
                Say 'Secrets stored.'
            } finally {
                [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($bstr)
                $pw = $null
            }
        } else {
            Say 'To store the four GitHub secrets, re-run with -SetSecrets (needs gh auth login).'
        }
    }

    'Binary' {
        Need gh 'winget install GitHub.cli, then gh auth login'
        $run = & gh run list --repo $Repo --workflow 'CapSnap server' --branch main --status success --limit 1 --json databaseId -q '.[0].databaseId'
        if (-not $run) { throw 'no successful CapSnap server run on main found' }
        $dist = Join-Path $root 'dist'
        New-Item -ItemType Directory -Force $dist | Out-Null
        Say "Downloading the artifact from run $run"
        & gh run download $run --repo $Repo -n capsnap-server-x86_64-linux-musl -D $dist
        if ($LASTEXITCODE -ne 0) { throw 'download failed' }
        $sha = Get-ChildItem $dist -Recurse -Filter 'capsnap-server.sha256' | Select-Object -First 1
        $bin = Get-ChildItem $dist -Recurse -Filter 'capsnap-server' | Select-Object -First 1
        if (-not $sha -or -not $bin) { throw 'the artifact did not contain capsnap-server and capsnap-server.sha256' }
        $want = ((Get-Content $sha.FullName -Raw) -split '\s+')[0].ToLower()
        $got = (Get-FileHash $bin.FullName -Algorithm SHA256).Hash.ToLower()
        if ($want -ne $got) { throw "checksum mismatch: expected $want, got $got" }
        Say "Verified: $($bin.FullName) ($got)"
    }

    'BoxPrep' {
        Need ssh 'Add the Windows optional feature "OpenSSH Client".'
        if (-not (Test-KeyLogin)) { throw "key login to $target does not work; run -Step SshKey -InstallKey first" }
        Say 'The box:'
        Remote 'cat /etc/os-release | head -n 2; uname -r'
        Warn 'This changes the box: package install, firewall, SSH settings. It does not touch your data.'

        if (Confirm-Step '1/3 Update and install nginx certbot ufw unattended-upgrades age?') {
            Remote 'export DEBIAN_FRONTEND=noninteractive; apt-get update -q && apt-get full-upgrade -y -q && apt-get install -y -q nginx certbot ufw unattended-upgrades age'
        }
        if (Confirm-Step '2/3 Firewall: allow ssh (22), http, https, then enable?') {
            $port = Read-Host 'SSH port on the box [22]'
            if (-not $port) { $port = '22' }
            if ($port -notmatch '^\d+$') { throw 'port must be a number' }
            Remote "ufw allow $port/tcp && ufw allow 80/tcp && ufw allow 443/tcp && ufw --force enable"
        }
        if (Confirm-Step '3/3 Key-only logins (turns OFF password login)? Your key login was just verified.') {
            Remote "printf '%s\n' 'PasswordAuthentication no' 'KbdInteractiveAuthentication no' 'PermitRootLogin prohibit-password' > /etc/ssh/sshd_config.d/10-key-only.conf && sshd -t && systemctl restart ssh"
            if (Test-KeyLogin) { Say 'A fresh key login still works.' } else { Warn 'A fresh key login failed. Keep your current session and fix it from the ZAP console before closing anything.' }
        }
        Say 'Next: -Step Deploy'
    }

    'Deploy' {
        Need scp 'Add the Windows optional feature "OpenSSH Client".'
        if (-not (Test-KeyLogin)) { throw "key login to $target does not work; run -Step SshKey -InstallKey first" }
        $bin = Get-ChildItem (Join-Path $root 'dist') -Recurse -Filter 'capsnap-server' -ErrorAction SilentlyContinue | Select-Object -First 1
        if (-not $bin) { throw 'no verified binary in dist\; run -Step Binary first' }

        Say 'Copying the deploy kit and the binary to /root/capsnap-deploy'
        Remote 'rm -rf /root/capsnap-deploy && mkdir -p /root/capsnap-deploy'
        & scp -r -o BatchMode=yes (Join-Path $root 'deploy\*') "${target}:/root/capsnap-deploy/"
        if ($LASTEXITCODE -ne 0) { throw 'scp of the deploy kit failed' }
        & scp -o BatchMode=yes $bin.FullName "${target}:/root/capsnap-deploy/capsnap-server"
        if ($LASTEXITCODE -ne 0) { throw 'scp of the binary failed' }
        # Windows checkouts can leave CRLF in scripts; strip it on the box.
        Remote "find /root/capsnap-deploy -type f \( -name '*.sh' -o -name '*.in' -o -name '*.service' -o -name '*.timer' -o -name '*.conf' -o -path '*/bin/*' \) -exec sed -i 's/\r$//' {} + && chmod +x /root/capsnap-deploy/*.sh /root/capsnap-deploy/bin/*"

        Say "Preflight for $Domain (read-only). Every line should be OK, with DNS pointing at this box:"
        Remote "/root/capsnap-deploy/preflight.sh $Domain"

        if (Confirm-Step "Install CapSnap for $Domain on the box now?") {
            Remote "/root/capsnap-deploy/install.sh --domain $Domain --binary /root/capsnap-deploy/capsnap-server"
            Say 'Next: -Step Smoke'
        }
    }

    'Smoke' {
        $bash = (Get-Command bash -ErrorAction SilentlyContinue)
        if (-not $bash) { $bash = Get-Item "$env:ProgramFiles\Git\bin\bash.exe" -ErrorAction SilentlyContinue }
        if (-not $bash) { throw 'bash not found; install Git for Windows (it provides Git Bash), or run smoke.sh in WSL.' }
        $exe = if ($bash -is [System.Management.Automation.ApplicationInfo]) { $bash.Source } else { $bash.FullName }
        & $exe (Join-Path $root 'deploy\smoke.sh') $Domain
        if ($LASTEXITCODE -ne 0) { throw 'smoke.sh reported a failure' }
    }
}

exit 0
