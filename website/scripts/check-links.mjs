import { access, readFile, readdir } from "node:fs/promises";
import { dirname, extname, join, relative, resolve } from "node:path";

const outputRoot = resolve("dist");
const siteBase = "/canis-minor";
const htmlFiles = [];

async function walk(directory) {
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) await walk(path);
    else if (entry.isFile() && extname(entry.name) === ".html")
      htmlFiles.push(path);
  }
}

function outputPathFor(urlPath) {
  let path = decodeURIComponent(urlPath);
  if (!path.startsWith(`${siteBase}/`) && path !== siteBase) return null;
  path = path.slice(siteBase.length).replace(/^\//, "");
  if (path === "") return join(outputRoot, "index.html");
  if (path.endsWith("/")) return join(outputRoot, path, "index.html");
  return extname(path)
    ? join(outputRoot, path)
    : join(outputRoot, `${path}.html`);
}

function resolveReference(sourceFile, reference) {
  const sourceRoute = `/${relative(outputRoot, dirname(sourceFile)).replaceAll("\\", "/")}/`;
  const resolved = new URL(
    reference,
    `https://local.invalid${siteBase}${sourceRoute}`,
  );
  return {
    target: outputPathFor(resolved.pathname),
    fragment: decodeURIComponent(resolved.hash.slice(1)),
  };
}

await walk(outputRoot);
const failures = [];
const attributePattern = /\b(?:href|src)=["']([^"']+)["']/g;
const htmlCache = new Map();

async function hasFragment(target, fragment) {
  if (!fragment) return true;
  let html = htmlCache.get(target);
  if (!html) {
    html = await readFile(target, "utf8");
    htmlCache.set(target, html);
  }
  const escaped = fragment.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return new RegExp(`\\bid=["']${escaped}["']`).test(html);
}

for (const file of htmlFiles) {
  const html = await readFile(file, "utf8");
  for (const match of html.matchAll(attributePattern)) {
    const reference = match[1];
    if (/^(?:https?:|mailto:|tel:|data:|javascript:)/.test(reference)) continue;
    const { target, fragment } = resolveReference(file, reference);
    if (!target) {
      failures.push(
        `${relative(outputRoot, file)}: link escapes ${siteBase}: ${reference}`,
      );
      continue;
    }
    try {
      await access(target);
    } catch {
      failures.push(`${relative(outputRoot, file)}: missing ${reference}`);
      continue;
    }
    if (!(await hasFragment(target, fragment))) {
      failures.push(
        `${relative(outputRoot, file)}: missing fragment ${reference}`,
      );
    }
  }
}

if (failures.length > 0) {
  console.error(failures.join("\n"));
  process.exitCode = 1;
} else {
  console.log(
    `Checked ${htmlFiles.length} HTML files; all internal links resolve.`,
  );
}
