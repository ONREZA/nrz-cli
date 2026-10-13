#!/usr/bin/env bash
set -euo pipefail

fail() { echo "$1" >&2; exit 1; }
if [[ "${RELEASE_HOLD_DRAFT:-false}" == true ]]; then
  [[ -z "${RELEASE_RESUME_VERSION:-}" && -z "${RELEASE_RETAG_VERSION:-}" ]] || fail 'Hold cannot be combined with resume or npm retag'
fi
if [[ -n "${RELEASE_RESUME_VERSION:-}" ]]; then
  [[ -z "${RELEASE_EXPLICIT_VERSION:-}" && -z "${RELEASE_RETAG_VERSION:-}" ]] || fail 'Resume cannot prepare a version or retag npm'
  [[ "${RELEASE_EXPECTED_REVISION:-}" =~ ^[a-f0-9]{40}$ ]] || fail 'Resume requires the exact tag revision'
  [[ "${RELEASE_EXPECTED_CHECKSUMS_SHA256:-}" =~ ^[a-f0-9]{64}$ ]] || fail 'Resume requires the recorded hold checksum digest'
  if [[ "${RELEASE_CHANNEL:-stable}" == stable ]]; then
    [[ "$RELEASE_RESUME_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail 'Resume version must match the stable channel'
  else
    [[ "$RELEASE_RESUME_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+-beta\.[0-9]+$ ]] || fail 'Resume version must match the beta channel'
  fi
fi
