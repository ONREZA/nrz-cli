# @onreza/nrz

ONREZA platform CLI — dev, build, deploy.

## Install

```bash
npm install -g @onreza/nrz
```

After installation the `nrz` binary is available in your `$PATH`.

## Usage

```bash
nrz dev          # Start dev server with ONREZA runtime emulation
nrz build        # Validate build output and manifest
nrz deploy       # Deploy to ONREZA platform
nrz upgrade      # Self-update to the latest version
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
curl -fsSL https://raw.githubusercontent.com/onreza/nrz-cli/main/install.sh | bash
```

**Windows (PowerShell 7+):**
```powershell
iwr -useb https://raw.githubusercontent.com/onreza/nrz-cli/main/install.ps1 | iex
```

## Documentation

Full documentation and source code: [github.com/onreza/nrz-cli](https://github.com/onreza/nrz-cli)

## License

MIT
