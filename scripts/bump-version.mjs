import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const nextVersion = process.argv[2];

if (typeof nextVersion !== "string" || !/^\d+\.\d+\.\d+$/.test(nextVersion)) {
  console.error("Usage: node scripts/bump-version.mjs 0.2.0");
  process.exit(1);
}

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, "..");

const packageJsonPath = path.join(rootDir, "package.json");
const tauriConfPath = path.join(rootDir, "src-tauri", "tauri.conf.json");
const cargoTomlPath = path.join(rootDir, "src-tauri", "Cargo.toml");

const packageJson = JSON.parse(await readFile(packageJsonPath, "utf8"));
if (packageJson.version !== nextVersion) {
  packageJson.version = nextVersion;
  await writeFile(packageJsonPath, `${JSON.stringify(packageJson, null, 2)}\n`);
}

const tauriConf = JSON.parse(await readFile(tauriConfPath, "utf8"));
if (tauriConf.version !== nextVersion) {
  tauriConf.version = nextVersion;
  await writeFile(tauriConfPath, `${JSON.stringify(tauriConf, null, 2)}\n`);
}

const cargoToml = await readFile(cargoTomlPath, "utf8");
const updatedCargoToml = cargoToml.replace(
  /^version\s*=\s*".*?"$/m,
  `version = "${nextVersion}"`
);

if (updatedCargoToml !== cargoToml) {
  await writeFile(cargoTomlPath, updatedCargoToml);
}

console.log(`Bumped version to ${nextVersion}`);
