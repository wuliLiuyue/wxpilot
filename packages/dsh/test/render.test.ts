import assert from "node:assert/strict";
import test from "node:test";

import { renderResultText, truncateWithMarker } from "../src/render.js";

test("truncateWithMarker keeps short text untouched", () => {
  assert.equal(truncateWithMarker("short", 100), "short");
});

test("truncateWithMarker cuts long text and appends the CLI marker", () => {
  const text = "x".repeat(300);
  const truncated = truncateWithMarker(text, 100);
  assert.ok(truncated.startsWith("x".repeat(100)));
  assert.ok(truncated.endsWith(`\n[truncated, originalLength=300]`));
});

test("renders a successful JSON payload without stderr", () => {
  const text = renderResultText(
    { parsed: { ok: true, page: "pages/index/index" }, stdout: "", stderr: "", exitCode: 0 },
    16_000,
  );
  assert.equal(text, JSON.stringify({ ok: true, page: "pages/index/index" }, null, 2));
});

test("appends a non-duplicated stderr tail on failure", () => {
  const text = renderResultText(
    { parsed: { ok: false, error: "ref expired" }, stdout: "", stderr: "daemon gone", exitCode: 1 },
    16_000,
  );
  assert.match(text, /"error": "ref expired"/);
  assert.match(text, /stderr:\ndaemon gone/);
});

test("does not append stderr when the fallback already shows it", () => {
  const text = renderResultText(
    { parsed: null, stdout: "", stderr: "crash trace", exitCode: 101 },
    16_000,
  );
  assert.equal(text, "crash trace");
});

test("appends stderr even when it appears as a substring of the stdout fallback", () => {
  const text = renderResultText(
    { parsed: null, stdout: "error: wrapper error: boom", stderr: "boom", exitCode: 1 },
    16_000,
  );
  assert.match(text, /^error: wrapper error: boom/);
  assert.match(text, /stderr:\nboom/);
});

test("falls back to an exit-code line when both streams are empty", () => {
  const text = renderResultText(
    { parsed: null, stdout: "  ", stderr: "", exitCode: 3 },
    16_000,
  );
  assert.equal(text, "wxpilot exited with code 3");
});

test("prefers raw stdout over stderr when the output is not JSON", () => {
  const text = renderResultText(
    { parsed: null, stdout: "panic: boom", stderr: "note: run with RUST_BACKTRACE", exitCode: 101 },
    16_000,
  );
  assert.match(text, /^panic: boom/);
  assert.match(text, /stderr:\nnote: run with RUST_BACKTRACE/);
});
