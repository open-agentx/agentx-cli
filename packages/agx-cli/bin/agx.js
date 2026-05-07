#!/usr/bin/env node

const { existsSync } = require("node:fs");
const { dirname, join, resolve } = require("node:path");
const { spawnSync } = require("node:child_process");

const platformPackageNames = {
  "darwin-arm64": "agx-cli-darwin-arm64",
  "darwin-x64": "agx-cli-darwin-x64",
  "linux-arm64": "agx-cli-linux-arm64",
  "linux-x64": "agx-cli-linux-x64",
  "win32-arm64": "agx-cli-win32-arm64",
  "win32-x64": "agx-cli-win32-x64"
};

function resolveBinary() {
  if (process.env.AGX_BINARY_PATH && existsSync(process.env.AGX_BINARY_PATH)) {
    return process.env.AGX_BINARY_PATH;
  }

  const key = `${process.platform}-${process.arch}`;
  const packageName = platformPackageNames[key];
  const binaryName = process.platform === "win32" ? "agx.exe" : "agx";
  if (packageName) {
    try {
      return require.resolve(`${packageName}/bin/${binaryName}`);
    } catch {
      // Fall through to workspace development lookup.
    }
  }

  const workspaceBinary = resolve(
    dirname(__filename),
    "..",
    "..",
    "..",
    "target",
    "release",
    binaryName
  );
  if (existsSync(workspaceBinary)) {
    return workspaceBinary;
  }

  throw new Error(
    `No AGX native binary found for ${key}. Reinstall agx-cli or set AGX_BINARY_PATH.`
  );
}

const binary = resolveBinary();
const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

process.exit(result.status ?? 1);
