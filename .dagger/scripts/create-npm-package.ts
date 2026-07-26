#!/usr/bin/env bun
import { createHash } from "node:crypto";
import { chmodSync, copyFileSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
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
  checksums: Record<string, string>;
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

export function createPostinstall({ assets, checksums, version }: PostinstallOptions): string {
  return `#!/usr/bin/env node
import { platform, arch } from "node:process";
import https from "node:https";
import { createHash, randomUUID } from "node:crypto";
import { chmodSync, existsSync, lstatSync, mkdirSync, mkdtempSync, renameSync, rmSync } from "node:fs";
import { join, dirname } from "node:path";
import { createGunzip } from "node:zlib";
import { fileURLToPath } from "node:url";
import * as tar from "tar";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const MANIFEST = ${JSON.stringify({ assets, checksums, binName: "nrz", version }, null, 2)};

const platformKey = \`\${platform}-\${arch}\`;
const downloadUrl = MANIFEST.assets[platformKey];
const expectedSha256 = MANIFEST.checksums[platformKey];

if (!downloadUrl || !expectedSha256) {
  console.error(\`No binary available for \${platformKey}\`);
  console.error(\`Supported platforms: \${Object.keys(MANIFEST.assets).join(", ")}\`);
  process.exit(1);
}

const binDir = join(__dirname, "..", "bin");
mkdirSync(binDir, { recursive: true });
const stagingDir = mkdtempSync(join(binDir, ".nrz-install-"));
const sourceName = MANIFEST.sourceBinName || MANIFEST.binName;
const ext = platform === "win32" ? ".exe" : "";
const sourcePath = join(stagingDir, sourceName + ext);
const targetPath = join(binDir, MANIFEST.binName + ext);
const backupPath = join(binDir, \`.\${MANIFEST.binName}\${ext}.old-\${randomUUID()}\`);

function replaceInstalledBinary(candidatePath, targetPath, backupPath) {
  if (platform !== "win32" || !existsSync(targetPath)) {
    renameSync(candidatePath, targetPath);
    return;
  }

  const targetStat = lstatSync(targetPath);
  if (!targetStat.isFile() && !targetStat.isSymbolicLink()) {
    throw new Error(\`Refusing to replace non-file binary path: \${targetPath}\`);
  }

  renameSync(targetPath, backupPath);
  try {
    renameSync(candidatePath, targetPath);
  } catch (installError) {
    try {
      renameSync(backupPath, targetPath);
    } catch (restoreError) {
      throw new Error(
        \`Failed to install replacement (\${installError.message}) and restore previous binary (\${restoreError.message}); previous binary remains at \${backupPath}\`,
      );
    }
    throw new Error(
      \`Failed to install replacement; previous binary was restored: \${installError.message}\`,
    );
  }

  try {
    rmSync(backupPath, { force: true });
  } catch (cleanupError) {
    console.warn(
      \`Installed new binary but could not remove backup \${backupPath}: \${cleanupError.message}\`,
    );
  }
}

function download(url, expectedSha256, redirectCount = 0) {
  if (redirectCount > 5) {
    return Promise.reject(new Error("Too many redirects"));
  }

  return new Promise((resolve, reject) => {
    const parsed = new URL(url);
    if (parsed.protocol !== "https:") {
      reject(new Error(\`Refusing non-HTTPS download URL: \${url}\`));
      return;
    }

    const request = https.get(url, (res) => {
      if ([301, 302, 307, 308].includes(res.statusCode)) {
        const location = res.headers.location;
        if (!location) {
          reject(new Error("Redirect without location header"));
          return;
        }
        res.resume();
        download(new URL(location, url).toString(), expectedSha256, redirectCount + 1)
          .then(resolve)
          .catch(reject);
        return;
      }

      if (res.statusCode !== 200) {
        res.resume();
        reject(new Error(\`Download failed: HTTP \${res.statusCode}\`));
        return;
      }
      const contentLength = Number(res.headers["content-length"]);
      if (Number.isFinite(contentLength) && contentLength > 256 * 1024 * 1024) {
        res.resume();
        reject(new Error("Download exceeds 256 MiB"));
        return;
      }

      const chunks = [];
      let received = 0;
      res.on("data", (chunk) => {
        received += chunk.length;
        if (received > 256 * 1024 * 1024) {
          request.destroy(new Error("Download exceeds 256 MiB"));
          return;
        }
        chunks.push(chunk);
      });
      res.on("end", () => {
        const archive = Buffer.concat(chunks);
        const actualSha256 = createHash("sha256").update(archive).digest("hex");
        if (actualSha256 !== expectedSha256) {
          reject(new Error(
            \`SHA-256 mismatch: expected \${expectedSha256}, received \${actualSha256}\`,
          ));
          return;
        }
        const gunzip = createGunzip();
        const extract = tar.extract({ cwd: stagingDir, strip: 1 });
        gunzip.on("error", reject);
        extract.on("error", reject);
        extract.on("finish", resolve);
        gunzip.pipe(extract);
        gunzip.end(archive);
      });
      res.on("error", reject);
    });

    request.on("error", reject);
    request.setTimeout(30000, () => {
      request.destroy();
      reject(new Error("Download timeout"));
    });
  });
}

console.log(\`Installing \${MANIFEST.binName} for \${platformKey}...\`);

download(downloadUrl, expectedSha256)
  .then(() => {
    if (!existsSync(sourcePath)) {
      throw new Error(\`Archive did not contain \${MANIFEST.binName}\${ext}\`);
    }

    if (platform !== "win32") {
      try {
        chmodSync(sourcePath, 0o755);
      } catch {
      }
    }
    replaceInstalledBinary(sourcePath, targetPath, backupPath);
    console.log(\`Installed \${MANIFEST.binName} v\${MANIFEST.version}\`);
  })
  .catch((err) => {
    console.error(\`Failed to install binary: \${err.message}\`);
    console.error(\`URL: \${downloadUrl}\`);
    process.exitCode = 1;
  })
  .finally(() => {
    try {
      rmSync(stagingDir, { recursive: true, force: true });
    } catch (cleanupError) {
      console.warn(
        \`Could not remove staging directory \${stagingDir}: \${cleanupError.message}\`,
      );
    }
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
  const checksums = Object.fromEntries(
    platforms.map((platform) => {
      const archive = readFileSync(join("/dist", `nrz-${platform}.tar.gz`));
      return [platform, createHash("sha256").update(archive).digest("hex")];
    }),
  );
  writeFileSync(join(outDir, "package.json"), `${JSON.stringify(createPackageJson({ version, channel }), null, 2)}\n`);
  writeFileSync(
    join(scriptsDir, "postinstall.js"),
    createPostinstall({ assets, checksums, version }),
  );
  writeFileSync(join(binDir, "nrz.js"), createShim());
  chmodSync(join(scriptsDir, "postinstall.js"), 0o755);
  chmodSync(join(binDir, "nrz.js"), 0o755);
  copyFileSync("npm-README.md", join(outDir, "README.md"));
  copyFileSync("LICENSE", join(outDir, "LICENSE"));
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main();
}
