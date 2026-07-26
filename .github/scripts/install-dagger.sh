#!/bin/sh
set -eu

version="$(jq -r '.engineVersion | sub("^v"; "")' dagger.json)"
case "$version" in
  0.21.4)
    expected_sha256="4db2f807b67e3160fb110bb1e088b14c516594b8d287114a39c980f2485e9672"
    ;;
  *)
    echo "No reviewed Dagger checksum for version $version" >&2
    exit 1
    ;;
esac

if [ "$(uname -s)" != "Linux" ] || [ "$(uname -m)" != "x86_64" ]; then
  echo "Unsupported Dagger bootstrap platform: $(uname -s)/$(uname -m)" >&2
  exit 1
fi

bin_dir="${RUNNER_TEMP:?RUNNER_TEMP is required}/dagger"
archive="${RUNNER_TEMP}/dagger_v${version}_linux_amd64.tar.gz"
url="https://github.com/dagger/dagger/releases/download/v${version}/dagger_v${version}_linux_amd64.tar.gz"
mkdir -p "$bin_dir"
curl --proto '=https' --tlsv1.2 --fail --location --silent --show-error \
  --output "$archive" "$url"
printf '%s  %s\n' "$expected_sha256" "$archive" | sha256sum --check --status
tar -xzf "$archive" -C "$bin_dir" dagger
chmod 0755 "$bin_dir/dagger"
printf '%s\n' "$bin_dir" >> "${GITHUB_PATH:?GITHUB_PATH is required}"
"$bin_dir/dagger" version
