# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Add integration tests and restructure test suite (tests)

- Add self-update command (upgrade)


### CI/CD

- Add GitHub Actions workflow for releases

- Add PowerShell install script for Windows

- Add git-cliff for automated changelog generation

- Add commitlint and lefthook for commit message validation

- Replace git-cliff-action with cargo install


### Changed

- Scaffold nrz CLI project

- Fix formatting with cargo fmt

- Fix formatting and update lefthook hooks


### Documentation

- Update CLAUDE.md with upgrade command and architecture

- Add project README (readme)


### Fixed

- Add Default impl and allow dead_code for unused fields (clippy)


### Performance

- Optimize release builds and add cleanup for upgrade



