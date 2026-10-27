import { execFileSync } from "node:child_process";
import { cpSync, copyFileSync, mkdirSync, rmSync } from "node:fs";

const dist = "msix/dist";
const release = "src-tauri/target/release";

rmSync(dist, { recursive: true, force: true });
mkdirSync(dist, { recursive: true });

for (const exe of ["spaces.exe", "hook_helper.exe"]) {
  copyFileSync(`${release}/${exe}`, `${dist}/${exe}`);
}

copyFileSync("msix/Package.appxmanifest", `${dist}/Package.appxmanifest`);
cpSync("msix/Assets", `${dist}/Assets`, { recursive: true });

console.log(`staged -> ${dist}`);

execFileSync("winapp", ["pack", dist, "--no-sign"], { stdio: "inherit" });
