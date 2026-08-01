#!/usr/bin/env node

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";

import { buildCliArgv, OPERATIONS } from "./command.js";
import { invokeWxpilot } from "./cli.js";

const server = new McpServer({
  name: "wxpilot-mcp",
  version: "0.2.0",
});

const PARAMETER_HINTS = [
  "Parameter keys: start(projectPath, cliPath?, appid?, autoPort?, proxy?, https?), connect(wsEndpoint?),",
  "goto(route, timeoutMs?), view(full?, compact?, depth?, limit?), find(query, all?, limit?),",
  "wait(reference, timeoutMs?), tap/read(reference), type(reference, text), scroll(reference, direction),",
  "assert(reference, expected), state(path?, full?), shot(path?, base64?), run(js), wx(method, args?),",
  "net.start(port?, https?), net.log(filter?, limit?, withBody?), net.mock(url, jsonFile, method?),",
  "net.unmock(url), net.stop, net.clear, and net.install-ca.",
].join(" ");

let operationQueue: Promise<void> = Promise.resolve();

server.registerTool(
  "wxpilot_execute",
  {
    title: "Execute wxpilot operation",
    description: [
      "Execute one whitelisted wxpilot CLI operation through the local wxpilot binary.",
      `Supported operations: ${OPERATIONS.join(", ")}.`,
      PARAMETER_HINTS,
      "start requires params.projectPath and never performs interactive project selection.",
      "Navigation or a new view/find invalidates old %N references; refresh them before interaction.",
      "Results are returned as CLI text so large or truncated CLI output remains visible.",
    ].join(" "),
    inputSchema: {
      operation: z.enum(OPERATIONS),
      params: z.record(z.string(), z.unknown()).optional(),
    },
  },
  async ({ operation, params }) =>
    enqueue(async () => {
      try {
        const argv = buildCliArgv(operation, params);
        const result = await invokeWxpilot(argv);
        return {
          content: [{ type: "text", text: result.text }],
          isError: !result.ok,
        };
      } catch (error: unknown) {
        return {
          content: [
            {
              type: "text",
              text: error instanceof Error ? error.message : String(error),
            },
          ],
          isError: true,
        };
      }
    }),
);

const transport = new StdioServerTransport();
transport.onerror = (error: Error) => {
  console.error(`wxpilot MCP transport error: ${error.message}`);
};

try {
  await server.connect(transport);
} catch (error: unknown) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}

function enqueue<T>(task: () => Promise<T>): Promise<T> {
  const result = operationQueue.then(task, task);
  operationQueue = result.then(
    () => undefined,
    () => undefined,
  );
  return result;
}
