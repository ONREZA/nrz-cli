#!/usr/bin/env bun
import { chmodSync, copyFileSync, mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

const outDir = "/out/npm";
const binDir = join(outDir, "bin");
const scriptsDir = join(outDir, "scripts");
const platforms = ["linux-x64", "darwin-x64", "darwin-arm64", "win32-x64"];

interface NpmPackageOptions {
  version: string;
  channel: string;
}

interface PostinstallOptions {
  assets: Record<string, string>;
  version: string;
}

interface ReleasePackageJson {
  name: string;
  version: string;
  private: boolean;
  type: "module";
  description: string;
  files: string[];
  scripts: {
    postinstall: string;
  };
  bin: {
    nrz: string;
  };
  dependencies: Record<string, string>;
  repository: {
    type: string;
    url: string;
  };
  publishConfig: {
    tag: string;
  };
}

export function releaseAssets(tag: string): Record<string, string> {
  return Object.fromEntries(
    platforms.map((platform) => [
      platform,
      `https://github.com/ONREZA/nrz-cli/releases/download/${tag}/nrz-${platform}.tar.gz`,
    ]),
  );
}

export function createPackageJson({ version, channel }: NpmPackageOptions): ReleasePackageJson {
  return {
    name: "@onreza/nrz",
    version,
    private: false,
    type: "module",
    description: "ONREZA platform CLI - dev, build, deploy",
    files: ["bin", "scripts", "README.md", "LICENSE"],
    scripts: {
      postinstall: "node scripts/postinstall.js",
    },
    bin: {
      nrz: "bin/nrz.js",
    },
    dependencies: {
      tar: "^7.0.0",
    },
    repository: {
      type: "git",
      url: "https://github.com/ONREZA/nrz-cli",
    },
    publishConfig: {
      tag: channel === "stable" ? "latest" : channel,
    },
  };
}

export function createPostinstall({ assets, version }: PostinstallOptions): string {
  return `#!/usr/bin/env node
import { platform, arch } from "node:process";
import https from "node:https";
import http from "node:http";
import { chmodSync, mkdirSync, renameSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { createGunzip } from "node:zlib";
import { fileURLToPath } from "node:url";
import * as tar from "tar";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const MANIFEST = ${JSON.stringify({ assets, binName: "nrz", version }, null, 2)};

const platformKey = \`\${platform}-\${arch}\`;
const downloadUrl = MANIFEST.assets[platformKey];

if (!downloadUrl) {
  console.error(\`No binary available for \${platformKey}\`);
  console.error(\`Supported platforms: \${Object.keys(MANIFEST.assets).join(", ")}\`);
  process.exit(1);
}

const binDir = join(__dirname, "..", "bin");
mkdirSync(binDir, { recursive: true });

function download(url, redirectCount = 0) {
  if (redirectCount > 5) {
    return Promise.reject(new Error("Too many redirects"));
  }

  return new Promise((resolve, reject) => {
    const protocol = url.startsWith("https") ? https : http;

    const request = protocol.get(url, (res) => {
      if ([301, 302, 307, 308].includes(res.statusCode)) {
        const location = res.headers.location;
        if (!location) {
          reject(new Error("Redirect without location header"));
          return;
        }
        download(location, redirectCount + 1).then(resolve).catch(reject);
        return;
      }

      if (res.statusCode !== 200) {
        reject(new Error(\`Download failed: HTTP \${res.statusCode}\`));
        return;
      }

      res
        .pipe(createGunzip())
        .pipe(tar.extract({ cwd: binDir, strip: 1 }))
        .on("finish", resolve)
        .on("error", reject);
    });

    request.on("error", reject);
    request.setTimeout(30000, () => {
      request.destroy();
      reject(new Error("Download timeout"));
    });
  });
}

console.log(\`Installing \${MANIFEST.binName} for \${platformKey}...\`);

download(downloadUrl)
  .then(() => {
    const sourceName = MANIFEST.sourceBinName || MANIFEST.binName;
    const ext = platform === "win32" ? ".exe" : "";
    const sourcePath = join(binDir, sourceName + ext);
    const targetPath = join(binDir, MANIFEST.binName + ext);

    if (sourceName !== MANIFEST.binName && existsSync(sourcePath)) {
      renameSync(sourcePath, targetPath);
    }

    if (platform !== "win32") {
      try {
        chmodSync(targetPath, 0o755);
      } catch {
      }
    }
    console.log(\`Installed \${MANIFEST.binName} v\${MANIFEST.version}\`);
  })
  .catch((err) => {
    console.error(\`Failed to install binary: \${err.message}\`);
    console.error(\`URL: \${downloadUrl}\`);
    process.exit(1);
  });
`;
}

function createShim(): string {
  return `#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import process from "node:process";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const ext = process.platform === "win32" ? ".exe" : "";
const binPath = join(__dirname, "nrz" + ext);

try {
  execFileSync(binPath, process.argv.slice(2), { stdio: "inherit" });
} catch (err) {
  if (err.status !== undefined) {
    process.exit(err.status);
  }
  throw err;
}
`;
}

function main(): void {
  const version = process.env.NRZ_RELEASE_VERSION;
  const tag = process.env.NRZ_RELEASE_TAG;
  const channel = process.env.NRZ_RELEASE_CHANNEL || "stable";

  if (!version || !tag) {
    throw new Error("NRZ_RELEASE_VERSION and NRZ_RELEASE_TAG are required");
  }

  mkdirSync(binDir, { recursive: true });
  mkdirSync(scriptsDir, { recursive: true });

  const assets = releaseAssets(tag);
  writeFileSync(join(outDir, "package.json"), `${JSON.stringify(createPackageJson({ version, channel }), null, 2)}\n`);
  writeFileSync(join(scriptsDir, "postinstall.js"), createPostinstall({ assets, version }));
  writeFileSync(join(binDir, "nrz.js"), createShim());
  chmodSync(join(scriptsDir, "postinstall.js"), 0o755);
  chmodSync(join(binDir, "nrz.js"), 0o755);
  copyFileSync("npm-README.md", join(outDir, "README.md"));
  copyFileSync("LICENSE", join(outDir, "LICENSE"));
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main();
}
