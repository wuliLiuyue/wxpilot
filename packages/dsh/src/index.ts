/**
 * Native dsh bundle plugin exposing the wxpilot Rust CLI as one model-facing
 * `wxpilot` tool. The tool layer owns schemas, operation gating, and result
 * rendering; the `ctx.subprocess` seam owns spawn execution, process-tree
 * termination, and environment scrubbing. Operations are serialized through
 * one queue because the CLI drives a stateful DevTools session (`view` assigns
 * the `%N` refs that later operations consume).
 *
 * Namespace plugin (named exports, no default export).
 * @module @wxpilot/dsh
 */

import type { Context } from "@deepseek-ai/cordis";
import type {} from "@deepseek-ai/dsh-subprocess";
import type {} from "@deepseek-ai/dsh-system-prompt";
import z from "@deepseek-ai/schemastery";
import { defineTool } from "@deepseek-ai/dsh-tools";
import { buildCliArgv, OPERATIONS } from "@wxpilot/shared";

import {
  createOperationQueue,
  runWxpilot,
  WxpilotToolError,
  type WxpilotProcessDeps,
  type WxpilotRunResult,
} from "./process.js";

/** Cordis plugin name for loader diagnostics. */
export const name = "tool-wxpilot";

/** Services required by this plugin. */
export const inject = ["tools", "systemPrompt", "subprocess"];

/** Operations that evaluate JavaScript inside the mini-program. */
const JS_EVAL_OPERATIONS: ReadonlySet<string> = new Set(["run", "wx"]);

/** Default per-call timeout budget (ms), covering daemon startup handshakes. */
export const DEFAULT_WXPILOT_TOOL_TIMEOUT_MS = 120_000;

/** Default model-facing result budget (characters). */
export const DEFAULT_WXPILOT_MAX_RESULT_CHARS = 16_000;

/** The stable system-prompt guidance positioning wxpilot in the workflow. */
export const WXPILOT_PROMPT_TEXT = [
  "Use the wxpilot tool when a WeChat mini-program project must run in WeChat DevTools: launching it, inspecting pages, interacting, asserting state, or capturing and mocking network traffic.",
  "Typical flow: start (projectPath is required — pass the project root that contains the mini-program build output), view to list interactive elements as %N refs, then tap/type/scroll/read on those refs.",
  "goto/back/reload invalidate every %N ref; call view again before interacting. Prefer find over scanning full views.",
  "Network capture requires a proxy session: start with proxy=true (https=true adds HTTPS decryption; first use needs net.install-ca and a DevTools restart).",
].join(" ");

/** Per-operation parameter hints, shared with the `@wxpilot/mcp` adapter. */
const PARAMETER_HINTS = [
  "start(projectPath, cliPath?, appid?, autoPort?, proxy?, https?), connect(wsEndpoint?),",
  "goto(route, timeoutMs?), view(full?, compact?, depth?, limit?), find(query, all?, limit?),",
  "wait(reference, timeoutMs?), tap/read(reference), type(reference, text),",
  "scroll(reference, direction up|down), assert(reference, expected),",
  "state(path?, full?), shot(path?, base64?),",
  "net.start(port?, https?), net.log(filter?, limit?, withBody?),",
  "net.mock(url, jsonFile, method?), net.unmock(url), net.stop, net.clear, net.install-ca.",
].join(" ");

/** Plugin configuration. */
export interface Config {
  /** Binary name or absolute path resolved through the subprocess seam (default `wxpilot`; `WXPILOT_BIN` overrides). */
  binary?: string;
  /** Cooperative tool-call timeout budget (ms), enforced by `dsh-tool-call-timeout-policy`. */
  timeoutMs?: number;
  /** Maximum retained characters of one rendered result. */
  maxResultChars?: number;
  /** Expose the `run` (JS evaluation) and `wx` (wx API call) operations (default false). */
  enableJsEval?: boolean;
}

export const Config: z<Config> = z.object({
  binary: z.string().default("wxpilot"),
  timeoutMs: z.number().default(DEFAULT_WXPILOT_TOOL_TIMEOUT_MS),
  maxResultChars: z.number().default(DEFAULT_WXPILOT_MAX_RESULT_CHARS),
  enableJsEval: z.boolean().default(false),
});

type ResolvedConfig = Required<Config>;

/**
 * Register the `wxpilot` tool and its system-prompt guidance.
 * @param ctx - the plugin context (must inject `tools`, `systemPrompt`, `subprocess`).
 * @param config - the resolved plugin configuration.
 */
export function apply(ctx: Context, config: Config): void {
  const resolved = config as ResolvedConfig;
  const operations = OPERATIONS.filter(
    (operation) => resolved.enableJsEval || !JS_EVAL_OPERATIONS.has(operation),
  );
  const enqueue = createOperationQueue();
  const deps: WxpilotProcessDeps = {
    resolveExecutable: (command, env, signal) =>
      ctx.subprocess.resolveExecutable(command, env, signal),
    spawn: (spec) => ctx.subprocess.spawn(spec),
  };

  ctx.systemPrompt.section({
    name: "tool:wxpilot",
    order: 113,
    text: WXPILOT_PROMPT_TEXT,
  });

  ctx.tools.register(
    defineTool({
      name: "wxpilot",
      description: buildToolDescription(operations, resolved.enableJsEval),
      parameters: {
        operation: {
          type: "string",
          required: true,
          enum: [...operations],
          description: "The wxpilot CLI operation to execute.",
        },
        params: {
          type: "object",
          additionalProperties: true,
          description: "Parameters for the operation; see the parameter hints.",
        },
      },
      output: {
        schema: {
          type: "object",
          additionalProperties: false,
          properties: {
            ok: {
              type: "boolean",
              required: true,
              description: "Whether the CLI reported success.",
            },
            exitCode: {
              oneOf: [{ type: "integer" }, { type: "null" }],
              required: true,
              description: "CLI exit code; null when killed by a signal.",
            },
            data: {
              type: "json",
              required: true,
              description: "Parsed CLI JSON payload, or null when the output was not JSON.",
            },
            text: {
              type: "string",
              required: true,
              description:
                "Model-facing rendering; may end with [truncated, originalLength=N].",
            },
          },
        },
        render: (_args, value) => [{ type: "text", text: value.text }],
      },
      timeoutMs: resolved.timeoutMs,
      presentCall(args) {
        return {
          card: "terminal",
          title: `wxpilot ${args.operation}`,
          description: "WeChat DevTools automation via the wxpilot CLI",
        };
      },
      async execute(args, exec) {
        return enqueue(async () => {
          // Argument-building and infrastructure failures (CommandValidationError,
          // WxpilotToolError, spawn failures) are reduced to the canonical
          // failed result instead of being thrown — mirroring the @wxpilot/mcp
          // adapter so the model always sees a structured outcome.
          try {
            const argv = buildCliArgv(args.operation, args.params);
            return await runWxpilot(deps, {
              binary: process.env.WXPILOT_BIN ?? resolved.binary,
              argv,
              cwd: exec.agent?.session.header.cwd ?? process.cwd(),
              signal: exec.signal,
              maxResultChars: resolved.maxResultChars,
            });
          } catch (error: unknown) {
            return failedResultFromError(error);
          }
        });
      },
    }),
  );
}

/** Compose the model-facing description from the gated operation list. */
function buildToolDescription(
  operations: readonly string[],
  jsEvalEnabled: boolean,
): string {
  return [
    "Drive the WeChat DevTools for a mini-program project through the local wxpilot CLI.",
    `Operations: ${operations.join(", ")}.`,
    `Parameter keys: ${PARAMETER_HINTS}`,
    ...(jsEvalEnabled
      ? ["Additionally: run(js) evaluates JavaScript in the mini-program; wx(method, args?) calls a wx API."]
      : []),
    "start requires params.projectPath and never performs interactive project selection.",
    "view assigns %N refs to interactive elements; goto/back/reload invalidate every ref, so call view again before interacting.",
    "Output is compact and may end with a [truncated, originalLength=N] marker.",
  ].join(" ");
}

/** Reduce a pre-execution failure to the canonical failed result value. */
function failedResultFromError(error: unknown): WxpilotRunResult {
  const text =
    error instanceof WxpilotToolError
      ? `${error.code}: ${error.message}`
      : error instanceof Error
        ? `${error.name}: ${error.message}`
        : String(error);
  return { ok: false, exitCode: null, data: null, text };
}
