/**
 * Write `latest.json`, the file "Check for updates" asks for (ADR-008).
 *
 *     node scripts/update-manifest.mjs <release-dir> <version> <owner/repo>
 *
 * Run by the release job over the folder of everything it is about to upload.
 * It reads the `.sig` file the bundler wrote beside each updatable artefact
 * and names, for each platform, the download and its signature. The URLs are
 * the ones the release *will* have once a person publishes the draft, which
 * is also the only moment the updater can see it: `releases/latest` skips
 * drafts.
 *
 * # When there is nothing to sign with
 *
 * A repository without `TAURI_SIGNING_PRIVATE_KEY` produces no signatures, and
 * then this writes nothing and says so. That is the same rule the code-signing
 * steps follow — an unsigned release that works beats a failed one — and the
 * copies it installs report "cannot update itself" rather than offering
 * something they could not verify.
 *
 * # Which file is which platform
 *
 * Read from the name the bundler gave it, and refused if it is not one of the
 * shapes below. A manifest that silently left a platform out would leave that
 * platform's users on an old version with nothing telling anyone.
 */

import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const [dir, version, repo] = process.argv.slice(2);
if (!dir || !version || !repo) {
  console.error("usage: update-manifest.mjs <release-dir> <version> <owner/repo>");
  process.exit(2);
}
if (!/^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(version)) {
  console.error(`update-manifest: ${version} is not a semantic version, which is what the updater compares`);
  process.exit(1);
}

/**
 * Each updatable artefact, by the end of its name → the keys the updater
 * looks up. The updater asks for `<os>-<arch>-<installer>` first and the bare
 * `<os>-<arch>` second, so the NSIS installer is also the fallback on Windows
 * — it is what a `-setup.exe` download installs, and the Windows installer
 * picks its method from the file it is handed.
 *
 * **Linux has no bare key, on purpose.** There the method comes from how the
 * running copy was installed, not from the file: a `.deb` install handed the
 * AppImage as a fallback would feed it to `dpkg`. So each Linux package is
 * offered only its own kind, and one that was not signed is offered nothing.
 */
const SHAPES = [
  { test: /_(x64|x86_64)-setup\.exe$/, keys: ["windows-x86_64-nsis", "windows-x86_64"] },
  { test: /_(x64|x86_64)_[A-Za-z-]+\.msi$/, keys: ["windows-x86_64-msi"] },
  { test: /_(amd64|x86_64)\.AppImage$/, keys: ["linux-x86_64-appimage"] },
  { test: /_(amd64|x86_64)\.deb$/, keys: ["linux-x86_64-deb"] },
  { test: /_(aarch64|arm64)\.app\.tar\.gz$/, keys: ["darwin-aarch64-app", "darwin-aarch64"] },
  { test: /_(x64|x86_64)\.app\.tar\.gz$/, keys: ["darwin-x86_64-app", "darwin-x86_64"] },
];

const signatures = readdirSync(dir).filter((f) => f.endsWith(".sig"));
if (signatures.length === 0) {
  console.log(
    "::warning::no updater signatures (TAURI_SIGNING_PRIVATE_KEY is not set) — " +
      "no latest.json, and copies of this release cannot update themselves",
  );
  process.exit(0);
}

const platforms = {};
const problems = [];
for (const sig of signatures.sort()) {
  const artefact = sig.slice(0, -".sig".length);
  if (!existsSync(join(dir, artefact))) {
    problems.push(`${sig} has no ${artefact} beside it`);
    continue;
  }
  // GitHub rewrites some characters in an asset's name, and a URL built from
  // the name on disk would then point at nothing.
  if (!/^[A-Za-z0-9._-]+$/.test(artefact)) {
    problems.push(`${artefact}: GitHub would rename this on upload`);
    continue;
  }
  const shape = SHAPES.find((s) => s.test.test(artefact));
  if (!shape) {
    problems.push(`${artefact}: signed, but not a shape this script knows the platform of`);
    continue;
  }
  const entry = {
    signature: readFileSync(join(dir, sig), "utf8").trim(),
    url: `https://github.com/${repo}/releases/download/v${version}/${artefact}`,
  };
  for (const key of shape.keys) {
    if (platforms[key]) problems.push(`two artefacts claim ${key}: ${platforms[key].url} and ${entry.url}`);
    platforms[key] = entry;
  }
}

// The three NFR-001 names. Each has to be offered something.
for (const needed of ["windows-x86_64", "darwin-aarch64", "linux-x86_64-appimage"]) {
  if (!platforms[needed]) problems.push(`nothing for ${needed}`);
}
// Said, not refused: the AppImage still updates, and a `.deb` install
// updating through its package manager is the ordinary Debian answer.
if (!platforms["linux-x86_64-deb"] && readdirSync(dir).some((f) => f.endsWith(".deb"))) {
  console.log("::notice::the .deb is not signed for the updater — .deb installs will not update themselves");
}

if (problems.length > 0) {
  for (const p of problems) console.error(`update-manifest: ${p}`);
  process.exit(1);
}

const manifest = {
  version,
  notes: `BibleCompose ${version}. What changed is in CHANGELOG.md.`,
  pub_date: new Date().toISOString(),
  platforms,
};
writeFileSync(join(dir, "latest.json"), JSON.stringify(manifest, null, 2) + "\n");
console.log(`latest.json: ${Object.keys(platforms).sort().join(", ")}`);
