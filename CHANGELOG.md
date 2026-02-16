# Changelog

All notable changes to this project will be documented in this file.

## [0.4.0] - 2026-02-16

### ⚠️ BREAKING CHANGES

- **cli:** add full CRUD subcommands for nrz projects ([8a5a086](https://github.com/ONREZA/nrz-cli/commit/8a5a086b021dd26f16ae10aacd113915afa76789))

## [0.3.1] - 2026-02-16

### 🐛 Bug Fixes

- **cli:** align API structs with actual server responses ([c38c7f7](https://github.com/ONREZA/nrz-cli/commit/c38c7f767a8b708b89b2fbb466a5f69c43c386b1))
- **cli:** migrate from removed /v1/user/projects to workspace-scoped API ([83091b0](https://github.com/ONREZA/nrz-cli/commit/83091b05cb6b2699a6cb9daa11afc3f954444268))

## [0.3.0] - 2026-02-15

### ✨ Features

- **cli:** add multi-workspace token support ([b3a90a7](https://github.com/ONREZA/nrz-cli/commit/b3a90a7e2271d9514b4c0ca42891acf278dec38e))

## [0.2.1] - 2026-02-15

### 🐛 Bug Fixes

- **cli:** fix upgrade command failing to find release binaries ([93f5da9](https://github.com/ONREZA/nrz-cli/commit/93f5da99477fc68f699ee07940cd42f309c1067d))

## [0.2.0] - 2026-02-15

### ✨ Features

- **cli:** add projects, deployments, logs, env, domains, rollback commands ([91778d9](https://github.com/ONREZA/nrz-cli/commit/91778d94e8bfa208b83bd269b31ffd4c2040da3a))

## [0.1.7] - 2026-02-11

### 🔧 Changed

- **deps:** fix typo package-lock.json ([f476255](https://github.com/ONREZA/nrz-cli/commit/f4762555bd391da05a041eaeccdf986d78d596fb))
- update install scripts, add MIT license, update deps ([f3719e8](https://github.com/ONREZA/nrz-cli/commit/f3719e856d310fb656452662d7ab394a331c6fc5))

## [0.1.6] - 2026-02-11

### 🔧 Changed

- **deps:** update onreza-release to 2.6.2 ([f0eb7bb](https://github.com/ONREZA/nrz-cli/commit/f0eb7bbeb643eb508099337239d41021c554823d))

## [0.1.5] - 2026-02-11

### 🔧 Changed

- **deps:** update onreza-release to 2.6.1 ([a536a72](https://github.com/ONREZA/nrz-cli/commit/a536a72a5145b6d000622a9f1aad6325ac6afc6e))

## [0.1.4] - 2026-02-11

### 👷 CI/CD

- **release:** update onreza-release to 2.6.0, remove bun and stdout parsing workarounds ([2eb0020](https://github.com/ONREZA/nrz-cli/commit/2eb0020192fa990204f569aae38d7e645c2ecca9))

## [0.1.3] - 2026-02-11

### 🐛 Bug Fixes

- **dev:** use cfg(unix) for SIGTERM, fix Windows build ([dc8fd1c](https://github.com/ONREZA/nrz-cli/commit/dc8fd1c41f1940d6c1564fe58bad0306df13fed7))

## [0.1.2] - 2026-02-11

### 👷 CI/CD

- **release:** parse onreza-release output for job outputs ([a54667e](https://github.com/ONREZA/nrz-cli/commit/a54667e581f76c95c03288c69ab9a6c461665112))
- **release:** pass GITHUB_TOKEN to onreza-release step ([3d6cba8](https://github.com/ONREZA/nrz-cli/commit/3d6cba856d1c9e2a0586e0850a1970b98c72063b))
- **release:** add setup-bun for onreza-release ([cd21de4](https://github.com/ONREZA/nrz-cli/commit/cd21de409dd9cebb8b871ce322676fe515b1c89a))
- **release:** track package-lock.json, remove old CHANGELOG.md ([2324f20](https://github.com/ONREZA/nrz-cli/commit/2324f20b5a7e2e4603bd8929c014b95cc33d1acb))
- **release:** remove NPM_TOKEN, use trusted publishing (OIDC) ([e6aca95](https://github.com/ONREZA/nrz-cli/commit/e6aca95a39e9a6fc385b3e594ddb18652dcf4387))
- **release:** migrate to onreza-release ([f4d6897](https://github.com/ONREZA/nrz-cli/commit/f4d6897e9abb04f3426ab3f80582aa9ed2a790fb))

### 🔧 Changed

- **release:** v0.1.1 [skip ci] ([1826a96](https://github.com/ONREZA/nrz-cli/commit/1826a96b69a9b112ccfb218092f0abbd4861e2d8))
- rename npm package to @onreza/nrz ([fadb213](https://github.com/ONREZA/nrz-cli/commit/fadb213fcb502e25fbac24b24b22b99a0d152772))

## [0.1.1] - 2026-02-11

### 👷 CI/CD

- **release:** pass GITHUB_TOKEN to onreza-release step ([3d6cba8](https://github.com/ONREZA/nrz-cli/commit/3d6cba856d1c9e2a0586e0850a1970b98c72063b))
- **release:** add setup-bun for onreza-release ([cd21de4](https://github.com/ONREZA/nrz-cli/commit/cd21de409dd9cebb8b871ce322676fe515b1c89a))
- **release:** track package-lock.json, remove old CHANGELOG.md ([2324f20](https://github.com/ONREZA/nrz-cli/commit/2324f20b5a7e2e4603bd8929c014b95cc33d1acb))
- **release:** remove NPM_TOKEN, use trusted publishing (OIDC) ([e6aca95](https://github.com/ONREZA/nrz-cli/commit/e6aca95a39e9a6fc385b3e594ddb18652dcf4387))
- **release:** migrate to onreza-release ([f4d6897](https://github.com/ONREZA/nrz-cli/commit/f4d6897e9abb04f3426ab3f80582aa9ed2a790fb))

### 🔧 Changed

- rename npm package to @onreza/nrz ([fadb213](https://github.com/ONREZA/nrz-cli/commit/fadb213fcb502e25fbac24b24b22b99a0d152772))

