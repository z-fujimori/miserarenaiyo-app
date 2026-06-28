import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const packageJsonPath = path.resolve(__dirname, "..", "package.json");

const packageJson = JSON.parse(await readFile(packageJsonPath, "utf8"));
const version = packageJson.version;

if (typeof version !== "string" || version.trim() === "") {
  throw new Error("package.json の version が見つかりません");
}

const commands = [
  `node scripts/bump-version.mjs ${version}`,
  "git add -A",
  `git commit -m "chore: release v${version}"`,
  `git tag v${version}`,
  `git push origin v${version}`,
];

console.log(commands.join("\n"));
