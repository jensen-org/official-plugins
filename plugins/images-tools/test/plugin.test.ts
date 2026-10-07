import { afterEach, describe, expect, test } from "bun:test";
import { start, TestHost } from "@jensen-org/plugin-sdk";
import Plugin from "../src/main.ts";

let host: TestHost;

afterEach(() => host?.close());

const INFO = { path: "img/a.png", format: "png", width: 64, height: 32, bytes: 4400 };

function report(overrides: Record<string, unknown>) {
  return {
    result: {
      ...INFO,
      before: INFO,
      removed: null,
      unchanged: false,
      ...overrides,
    },
    touched: [],
  };
}

async function boot(calls: Record<string, (input: never) => unknown> = {}) {
  host = new TestHost({
    respond: {
      "fs.stat": () => ({ kind: "file", size: 4400, modifiedMs: 1 }),
      "fs.readBytes": () => "AQID",
      "backend.call": ({ method, input }) => {
        const handler = calls[method];
        if (method === "info") return { result: INFO, touched: [] };
        return handler ? (handler(input as never) as never) : { result: {}, touched: [] };
      },
    },
  });
  const plugin = await start(Plugin as never, { connection: host.connection });
  await host.settle();
  return plugin;
}

function text(nodes: unknown): string {
  return JSON.stringify(nodes);
}

describe("the toolbar", () => {
  test("registers for the image viewer with the commands and the explorer menu", async () => {
    await boot();

    expect(host.callsTo("viewer.registerToolbar")).toEqual([
      { id: "images", kinds: ["image"], extensions: undefined },
    ]);
    expect(host.callsTo("commands.register").map((c) => c.id)).toEqual([
      "compress",
      "convert-webp",
      "convert-png",
      "convert-jpg",
    ]);
    expect(host.callsTo("ui.registerMenu")).toHaveLength(1);
  });

  test("renders every tool with the image's own size filled in", async () => {
    await boot();

    const out = text(await host.request("viewer.render", { toolbarId: "images", path: "img/a.png" }));

    for (const label of ["Resize", "Compress", "Round corners", "Convert"]) {
      expect(out).toContain(label);
    }
    expect(out).toContain('"value":"64"');
    expect(out).toContain('"value":"32"');
    expect(out).not.toContain('"value":"png"');
  });

  test("shows the error when the image cannot be read", async () => {
    host = new TestHost({
      respond: {
        "fs.stat": () => ({ kind: "file", size: 1, modifiedMs: 1 }),
        "backend.call": () => {
          throw new Error("a.png could not be decoded");
        },
      },
    });
    await start(Plugin as never, { connection: host.connection });

    const out = text(await host.request("viewer.render", { toolbarId: "images", path: "a.png" }));

    expect(out).toContain("could not be decoded");
  });
});

function handlerFor(out: unknown, label: string): string {
  const nodes = JSON.parse(text(out)) as unknown;
  const walk = (node: unknown): string | null => {
    if (Array.isArray(node)) {
      for (const child of node) {
        const found = walk(child);
        if (found) return found;
      }
      return null;
    }
    if (node && typeof node === "object") {
      const item = node as Record<string, unknown>;
      if (item.type === "button" && item.label === label) {
        return (item.onClick as { handler: string }).handler;
      }
      for (const value of Object.values(item)) {
        const found = walk(value);
        if (found) return found;
      }
    }
    return null;
  };
  const found = walk(nodes);
  if (!found) throw new Error(`no ${label} button`);
  return found;
}

describe("applying an operation", () => {
  test("converts, moves the open tab to the new file, and offers undo", async () => {
    await boot({
      convert: () =>
        report({ path: "img/a.webp", format: "webp", bytes: 1100, removed: "img/a.png" }),
    });
    const first = await host.request("viewer.render", { toolbarId: "images", path: "img/a.png" });

    await host.request("node.event", { handler: handlerFor(first, "Convert") });
    await host.settle();

    expect(host.callsTo("backend.call").filter((c) => c.method === "convert")).toEqual([
      { method: "convert", input: { path: "img/a.png", to: "webp" } },
    ]);
    expect(host.callsTo("editor.retarget")).toEqual([{ from: "img/a.png", to: "img/a.webp" }]);
    const after = text(await host.request("viewer.render", { toolbarId: "images", path: "img/a.webp" }));
    expect(after).toContain("Converted png to webp");
    expect(after).toContain("Undo");
  });

  test("resizes with the typed size and reports the saving", async () => {
    await boot({
      resize: () => report({ width: 32, height: 16, bytes: 1000 }),
    });
    const first = await host.request("viewer.render", { toolbarId: "images", path: "img/a.png" });

    await host.request("node.event", { handler: handlerFor(first, "Resize") });
    await host.settle();

    expect(host.callsTo("backend.call").find((c) => c.method === "resize")?.input).toEqual({
      path: "img/a.png",
      width: 64,
      height: 32,
      keepAspect: false,
    });
    expect(host.callsTo("editor.retarget")).toEqual([]);
  });

  test("says so, and records nothing, when the file could not get smaller", async () => {
    await boot({ compress: () => report({ unchanged: true }) });
    const first = await host.request("viewer.render", { toolbarId: "images", path: "img/a.png" });

    await host.request("node.event", { handler: handlerFor(first, "Compress") });
    await host.settle();

    const after = text(await host.request("viewer.render", { toolbarId: "images", path: "img/a.png" }));
    expect(after).toContain("Already as small as it gets");
    expect(after).not.toContain("Undo");
  });

  test("restores the original bytes, moves the tab back and deletes the converted file on undo", async () => {
    await boot({
      convert: () =>
        report({ path: "img/a.webp", format: "webp", bytes: 1100, removed: "img/a.png" }),
    });
    const first = await host.request("viewer.render", { toolbarId: "images", path: "img/a.png" });
    await host.request("node.event", { handler: handlerFor(first, "Convert") });
    await host.settle();
    const second = await host.request("viewer.render", { toolbarId: "images", path: "img/a.webp" });

    await host.request("node.event", { handler: handlerFor(second, "Undo") });
    await host.settle();

    const order = host.calls
      .map((call) => call.method)
      .filter((method) =>
        ["fs.writeBytes", "editor.retarget", "fs.delete"].includes(method),
      );
    expect(order).toEqual(["editor.retarget", "fs.writeBytes", "editor.retarget", "fs.delete"]);
    expect(host.callsTo("fs.writeBytes")).toEqual([{ path: "img/a.png", data: "AQID" }]);
    expect(host.callsTo("fs.delete")).toEqual([{ path: "img/a.webp", recursive: undefined }]);
  });
});
