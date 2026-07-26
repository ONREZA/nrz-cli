# @onreza/nrz

ONREZA platform CLI — dev, build, deploy.

## Install

```bash
npm install -g @onreza/nrz
npm install -g @onreza/nrz@beta  # prerelease channel
```

After installation the `nrz` binary is available in your `$PATH`.

## Usage

```bash
nrz dev          # Start dev server with ONREZA runtime emulation
nrz build        # Validate build output and manifest
nrz deploy       # Deploy to ONREZA platform
nrz upgrade      # Self-update to the latest version
nrz upgrade --channel beta
```

## Supported platforms

| Platform | Architecture |
|----------|-------------|
| Linux | x86_64 |
| macOS | x86_64, Apple Silicon |
| Windows | x86_64 |

## Alternative installation

**Linux/macOS:**
```bash
installer="$(mktemp)"
curl -fsSL https://raw.githubusercontent.com/onreza/nrz-cli/main/install.sh -o "$installer"
less "$installer"  # optional: review before running
bash "$installer"
rm -f "$installer"
```

**Windows (PowerShell 7+):**
```powershell
$installer = (New-TemporaryFile).FullName
iwr -useb https://raw.githubusercontent.com/onreza/nrz-cli/main/install.ps1 -OutFile $installer
Get-Content $installer  # optional: review before running
pwsh -NoProfile -File $installer
Remove-Item $installer
```

Both installers verify the downloaded archive against the SHA-256 manifest
published with the selected GitHub release.

## Documentation

Full documentation and source code: [github.com/onreza/nrz-cli](https://github.com/onreza/nrz-cli)

## License

MIT
