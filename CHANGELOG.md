# Changelog

All notable changes to this project will be documented in this file.

## [0.13.3] - 2026-02-21

### 🐛 Bug Fixes

- upload urls validation while bundleUploadUrl is set ([f7978de](https://github.com/ONREZA/nrz-cli/commit/f7978de85043a2aeb3025707585c79f9ce5852e3))

## [0.13.2] - 2026-02-21

### 🐛 Bug Fixes

- error messages while uploading bundle ([9e2c457](https://github.com/ONREZA/nrz-cli/commit/9e2c4576f3eca7f2f9a7797c4624ad49ed2bfc90))

## [0.13.1] - 2026-02-21

### 🐛 Bug Fixes

- **deploy:** improve PROCESS deploy reliability and error messages ([07f7449](https://github.com/ONREZA/nrz-cli/commit/07f7449bdb81e7bddc03a375846868c91d711faa))

### 📚 Documentation

- **config:** update JSON schema and add onreza.toml field reference ([588dd34](https://github.com/ONREZA/nrz-cli/commit/588dd341b88d281383305345ef9db83ae9ef8a26))

## [0.13.0] - 2026-02-20

### ✨ Features

- **cli:** improve SSR detection for Next.js, Nuxt, SvelteKit and add Astro SSR ([923e854](https://github.com/ONREZA/nrz-cli/commit/923e85478507baef17b454ed8ba39319d3a19eb1))
- **deploy:** resolve entry point for PROCESS deployments without adapter ([be15c42](https://github.com/ONREZA/nrz-cli/commit/be15c42da40677c1ab080dccf0a8873aba3cfa2f))
- **deploy:** support STATIC/PROCESS deploy without adapter manifest ([a1b2ce7](https://github.com/ONREZA/nrz-cli/commit/a1b2ce7378b5cbd6ad54b7c24dc09e410f7fa4a3))

### 🐛 Bug Fixes

- **build:** improve framework detection accuracy and output dir resolution ([16b428e](https://github.com/ONREZA/nrz-cli/commit/16b428eae093cffac1ba958e69ac9d70dd853a01))

## [0.12.0] - 2026-02-20

### ✨ Features

- add install step to nrz deploy ([fe59ac7](https://github.com/ONREZA/nrz-cli/commit/fe59ac7e062d40f2b62acfc9a0296c921b74a7d4))

## [0.11.1] - 2026-02-20

### 🐛 Bug Fixes

- **ci:** glibc -> musl ([6c83c85](https://github.com/ONREZA/nrz-cli/commit/6c83c8556c13c9d80d1a1c08f7fb20be96f56b34))

## [0.11.0] - 2026-02-20

### ✨ Features

- **deploy:** add tar.zst bundle upload for PROCESS deployments ([464b121](https://github.com/ONREZA/nrz-cli/commit/464b1217202a31d1d4e042762920cbc3caebfb55))
- **deploy:** add --resume-deployment flag for builder mode ([964b4f4](https://github.com/ONREZA/nrz-cli/commit/964b4f42bbc59aec3d8c5d03846e60af72cd209a))
- **cli:** add framework detection module and `nrz detect` command ([1343967](https://github.com/ONREZA/nrz-cli/commit/1343967fe52ef3295befb3f5e7f534ab516ceffd))
- **cli:** support multiple --env targets for env commands ([a96619c](https://github.com/ONREZA/nrz-cli/commit/a96619c34ed17508d5b9d0c52eaaf2ebab2423b3))

## [0.10.0] - 2026-02-18

### ✨ Features

- **config:** add [env.declarations] for env var visibility and validation ([aa6464d](https://github.com/ONREZA/nrz-cli/commit/aa6464d9defb8402c91e3e28543abc4c7220d047))

## [0.9.0] - 2026-02-18

### ✨ Features

- **deploy:** add auto-build step with --skip-build flag ([e1711e0](https://github.com/ONREZA/nrz-cli/commit/e1711e08b406cc161e518b29365ce80991678862))
- **dev:** add --alias, --inspect, --inspect-brk flags ([1599550](https://github.com/ONREZA/nrz-cli/commit/15995508719930fc660a75c55436cb99d4fbc0e2))
- **config:** add build.command and dev.aliases to onreza.toml ([2598826](https://github.com/ONREZA/nrz-cli/commit/259882629b9ec2305c7ddaa87e26c7cff4ea3853))
- **cli:** add `nrz env push` command ([a4c4c55](https://github.com/ONREZA/nrz-cli/commit/a4c4c55386e2edcdfcd878d2a9ab20b3222c0abe))

### 🔧 Changed

- **config:** update JSON schema with build.command and dev.aliases ([0752ad3](https://github.com/ONREZA/nrz-cli/commit/0752ad3ecfc83587b9ca74d93bb69a55b8dc562b))

## [0.8.2] - 2026-02-17

### 🐛 Bug Fixes

- **cli:** unwrap API envelope in response parsing ([b92624f](https://github.com/ONREZA/nrz-cli/commit/b92624fa99f08466598a68c4119f44fa89c22afa))

## [0.8.1] - 2026-02-17

### ♻️ Changed

- **config:** remove project.json, make init local-first, add JSON Schema ([ac3ea67](https://github.com/ONREZA/nrz-cli/commit/ac3ea67ce339713c9921f19e3c34fec99fc70fae))

## [0.8.0] - 2026-02-17

### ✨ Features

- **config:** add onreza.toml project configuration ([cf63448](https://github.com/ONREZA/nrz-cli/commit/cf634481aace38cf6b901489db874e5288b9013c))

### 🐛 Bug Fixes

- **ci:** add rustfmt and clippy components to pinned toolchain ([3c78804](https://github.com/ONREZA/nrz-cli/commit/3c78804d9f2d3eb26caec93f1f11bb06ed583155))

### 🎨 Changed

- **cli:** reformat for rustfmt 1.92 ([d8445d1](https://github.com/ONREZA/nrz-cli/commit/d8445d1bc32236458d4c91c38ad94c334c185e50))

## [0.7.0] - 2026-02-17

### ✨ Features

- **db:** add environment selection and remote db reset ([7f91a59](https://github.com/ONREZA/nrz-cli/commit/7f91a59a4865ab9d4c2cb1f713c1bfae0e65265f))
- **db:** add D1 migration system and nrz init command ([3d34b4e](https://github.com/ONREZA/nrz-cli/commit/3d34b4ede7204aab646b30fe26533eb31e2d18bd))

### 🔧 Changed

- **ci:** pin Rust toolchain to 1.92.0 ([a358002](https://github.com/ONREZA/nrz-cli/commit/a3580025e5bff3eb7de6f57e34e7f565c10b9fc4))

## [0.6.0] - 2026-02-16

### ✨ Features

- **deploy:** replace tar.gz uploads with flat per-file uploads ([e943a74](https://github.com/ONREZA/nrz-cli/commit/e943a74db8b1730bb691d089d0d2f07adfa2a583))

### 🔧 Changed

- **deps:** upgrade rusqlite 0.38, reqwest 0.13, console 0.16, indicatif 0.18 ([e2a0ecd](https://github.com/ONREZA/nrz-cli/commit/e2a0ecd2ad6390438168a5b112bf181331ddada2))

## [0.5.0] - 2026-02-16

### ✨ Features

- **db:** add --file, stdin, and multi-statement support to db execute ([3495216](https://github.com/ONREZA/nrz-cli/commit/3495216cba36d3c4485fce64dd8d8ee337434bed))

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

