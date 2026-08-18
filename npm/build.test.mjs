// Tests of the publishing glue. The platform list lives in three places that must agree, and the
// assembly must produce installable packages - both are cheap to check and expensive to get wrong.
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterAll, describe, expect, it } from "vitest";

import { TARGETS, build } from "./build.mjs";

const root = join(import.meta.dirname, "..");

// An explicit comparator, so the ordering never depends on the runtime's default.
const byName = (a, b) => a.localeCompare(b);

describe("the platform list", () => {
  it("matches the wrapper optionalDependencies", () => {
    const wrapper = JSON.parse(
      readFileSync(join(import.meta.dirname, "ai-watermark-guard", "package.json"), "utf8"),
    );
    const fromTargets = TARGETS.map(
      (target) => `@soroush.tech/ai-watermark-guard-${target.suffix}`,
    ).sort(byName);
    expect(Object.keys(wrapper.optionalDependencies).sort(byName)).toEqual(fromTargets);
  });

  it("matches the cd-publish workflow matrix", () => {
    const workflow = readFileSync(join(root, ".github", "workflows", "cd-publish.yml"), "utf8");
    const fromMatrix = [...workflow.matchAll(/- target: (\S+)/g)]
      .map((match) => match[1])
      .sort(byName);
    expect(TARGETS.map((target) => target.rust).sort(byName)).toEqual(fromMatrix);
  });
});

describe("build", () => {
  const scratch = mkdtempSync(join(tmpdir(), "aiwg-npm-"));
  const artifacts = join(scratch, "artifacts");
  const dist = join(scratch, "dist");
  afterAll(() => rmSync(scratch, { recursive: true, force: true }));

  for (const target of TARGETS) {
    const binary = target.os === "win32" ? "aiwg.exe" : "aiwg";
    mkdirSync(join(artifacts, target.rust), { recursive: true });
    writeFileSync(join(artifacts, target.rust, binary), `fake ${target.rust}`);
  }
  build(artifacts, "9.9.9", dist);

  it("writes one package per platform with the fields npm selects on", () => {
    for (const target of TARGETS) {
      const manifest = JSON.parse(readFileSync(join(dist, target.suffix, "package.json"), "utf8"));
      expect(manifest.name).toBe(`@soroush.tech/ai-watermark-guard-${target.suffix}`);
      expect(manifest.version).toBe("9.9.9");
      expect(manifest.os).toEqual([target.os]);
      expect(manifest.cpu).toEqual([target.cpu]);
      expect(manifest.libc).toEqual(target.libc ? [target.libc] : undefined);
    }
  });

  it("ships the binary under bin/", () => {
    expect(readFileSync(join(dist, "win32-x64", "bin", "aiwg.exe"), "utf8")).toBe(
      "fake x86_64-pc-windows-msvc",
    );
    expect(readFileSync(join(dist, "linux-x64-musl", "bin", "aiwg"), "utf8")).toBe(
      "fake x86_64-unknown-linux-musl",
    );
  });

  it("pins every optional dependency of the wrapper to the built version", () => {
    const manifest = JSON.parse(
      readFileSync(join(dist, "ai-watermark-guard", "package.json"), "utf8"),
    );
    expect(manifest.version).toBe("9.9.9");
    expect(Object.values(manifest.optionalDependencies)).toEqual(TARGETS.map(() => "9.9.9"));
  });
});
