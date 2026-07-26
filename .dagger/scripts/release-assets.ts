export const RELEASE_REPOSITORY = "ONREZA/nrz-cli";

export const RELEASE_PLATFORMS = [
  "linux-x64",
  "darwin-x64",
  "darwin-arm64",
  "win32-x64",
] as const;

export type ReleasePlatform = (typeof RELEASE_PLATFORMS)[number];

export function releaseAssetName(platform: ReleasePlatform): string {
  return `nrz-${platform}.tar.gz`;
}

export const REQUIRED_RELEASE_ASSETS = RELEASE_PLATFORMS.map(releaseAssetName);
