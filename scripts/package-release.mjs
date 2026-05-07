import { createHash } from "node:crypto";
import { copyFile, mkdir, readFile, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const platform = process.platform;
const arch = process.arch;
const binaryName = platform === "win32" ? "agx.exe" : "agx";
const binaryPath = join(repoRoot, "target", "release", binaryName);
const artifactDir = join(repoRoot, "release-artifacts");

if (!existsSync(binaryPath)) {
  throw new Error(`Missing release binary at ${binaryPath}. Run cargo build --workspace --release.`);
}

const binary = await readFile(binaryPath);
const sha256 = createHash("sha256").update(binary).digest("hex");
const versionResult = spawnSync(binaryPath, ["--version"], { encoding: "utf8" });
if (versionResult.status !== 0) {
  throw new Error(`Release binary failed --version: ${versionResult.stderr}`);
}

const version = versionResult.stdout.trim().split(/\s+/).at(-1);
const assetName = `agx-${platform}-${arch}${platform === "win32" ? ".exe" : ""}`;
const manifest = {
  generatedAt: new Date().toISOString(),
  packageName: "agx-cli",
  version,
  assets: [
    {
      arch,
      fileName: basename(binaryPath),
      name: assetName,
      os: platform,
      sha256,
      size: binary.byteLength
    }
  ]
};

await mkdir(artifactDir, { recursive: true });
await copyFile(binaryPath, join(artifactDir, assetName));
await writeFile(join(artifactDir, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
await writeFile(join(artifactDir, "SHA256SUMS"), `${sha256}  ${assetName}\n`);
await writeFile(join(artifactDir, `${assetName}.json`), `${JSON.stringify(manifest, null, 2)}\n`);
await writeFile(join(artifactDir, `${assetName}.sha256`), `${sha256}  ${assetName}\n`);
console.log(`Wrote release manifest for ${assetName}`);
