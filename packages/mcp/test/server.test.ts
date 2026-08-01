import assert from "node:assert/strict";
import { chmod, mkdtemp, rm, writeFile } from "node:fs/promises";
import { createInterface } from "node:readline";
import { spawn } from "node:child_process";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { once } from "node:events";
import test from "node:test";

interface RpcResult {
  serverInfo?: {
    name?: string;
  };
  capabilities?: {
    tools?: unknown;
  };
  tools?: Array<{
    name: string;
  }>;
  isError?: boolean;
  content?: Array<{
    type: string;
    text: string;
  }>;
}

interface RpcMessage {
  result?: RpcResult;
}

const packageRoot = resolve(process.cwd());
const serverEntry = join(packageRoot, "dist/index.js");

test("serves MCP initialize, tools/list, and tools/call over stdio", async () => {
  const directory = await mkdtemp(join(tmpdir(), "wxpilot-mcp-server-"));
  const fakeCli = join(directory, "wxpilot");
  await writeFile(
    fakeCli,
    [
      "#!/usr/bin/env node",
      "process.stdout.write(JSON.stringify({ ok: true, argv: process.argv.slice(2) }));",
      "",
    ].join("\n"),
  );
  await chmod(fakeCli, 0o755);

  const child = spawn(process.execPath, [serverEntry], {
    cwd: packageRoot,
    env: { ...process.env, WXPILOT_BIN: fakeCli },
    stdio: ["pipe", "pipe", "pipe"],
  });
  const reader = createInterface({ input: child.stdout });
  const pending: Array<(message: RpcMessage) => void> = [];
  const lines: RpcMessage[] = [];
  reader.on("line", (line: string) => {
    const message = JSON.parse(line) as RpcMessage;
    const resolveMessage = pending.shift();
    if (resolveMessage) {
      resolveMessage(message);
    } else {
      lines.push(message);
    }
  });

  const nextMessage = (): Promise<RpcMessage> => {
    if (lines.length > 0) {
      return Promise.resolve(lines.shift() as RpcMessage);
    }
    return new Promise((resolveMessage, reject) => {
      const timer = setTimeout(() => {
        reject(new Error("timed out waiting for MCP response"));
      }, 5000);
      pending.push((message) => {
        clearTimeout(timer);
        resolveMessage(message);
      });
    });
  };

  const send = (message: Record<string, unknown>): void => {
    child.stdin.write(`${JSON.stringify(message)}\n`);
  };

  try {
    send({
      jsonrpc: "2.0",
      id: 1,
      method: "initialize",
      params: {
        protocolVersion: "2025-06-18",
        capabilities: {},
        clientInfo: { name: "wxpilot-test", version: "0.0.0" },
      },
    });
    const initialized = await nextMessage();
    assert.equal(initialized.result?.serverInfo?.name, "wxpilot-mcp");
    assert.ok(initialized.result?.capabilities?.tools);

    send({ jsonrpc: "2.0", method: "notifications/initialized" });
    send({ jsonrpc: "2.0", id: 2, method: "tools/list", params: {} });
    const listed = await nextMessage();
    assert.deepEqual(
      listed.result?.tools?.map((tool) => tool.name),
      ["wxpilot_execute"],
    );

    send({
      jsonrpc: "2.0",
      id: 3,
      method: "tools/call",
      params: {
        name: "wxpilot_execute",
        arguments: { operation: "status", params: {} },
      },
    });
    const called = await nextMessage();
    assert.equal(called.result?.isError, false);
    const content = called.result?.content?.[0];
    assert.ok(content);
    assert.deepEqual(JSON.parse(content.text), {
      ok: true,
      argv: ["--json", "status"],
    });
  } finally {
    child.kill();
    await once(child, "exit").catch(() => undefined);
    reader.close();
    await rm(directory, { recursive: true, force: true });
  }
});
