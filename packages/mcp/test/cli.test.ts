import assert from "node:assert/strict";
import { access, chmod, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { invokeWxpilot } from "../src/cli.js";

test("invokes the CLI without shell interpolation", async () => {
  const directory = await mkdtemp(join(tmpdir(), "wxpilot-mcp-cli-"));
  const fakeCli = join(directory, "wxpilot");
  const marker = join(directory, "marker");
  await writeFile(
    fakeCli,
    [
      "#!/usr/bin/env node",
      "process.stdout.write(JSON.stringify(process.argv.slice(2)));",
      "",
    ].join("\n"),
  );
  await chmod(fakeCli, 0o755);

  try {
    const result = await invokeWxpilot(
      ["--json", "run", "--", `$(touch ${marker})`],
      { env: { ...process.env, WXPILOT_BIN: fakeCli } },
    );

    assert.equal(result.ok, true);
    assert.deepEqual(JSON.parse(result.text), [
      "--json",
      "run",
      "--",
      `$(touch ${marker})`,
    ]);
    await assert.rejects(access(marker));
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("returns stderr and a failure flag for CLI errors", async () => {
  const directory = await mkdtemp(join(tmpdir(), "wxpilot-mcp-cli-error-"));
  const fakeCli = join(directory, "wxpilot");
  await writeFile(
    fakeCli,
    [
      "#!/usr/bin/env node",
      "process.stderr.write('fake cli failure');",
      "process.exitCode = 7;",
      "",
    ].join("\n"),
  );
  await chmod(fakeCli, 0o755);

  try {
    const result = await invokeWxpilot(["--json", "status"], {
      env: { ...process.env, WXPILOT_BIN: fakeCli },
    });

    assert.equal(result.ok, false);
    assert.match(result.text, /fake cli failure/);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
