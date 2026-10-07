import { describe, expect, test } from "bun:test";
import {
  aspectHeight,
  aspectWidth,
  change,
  dimensions,
  extensionOf,
  human,
  isImage,
  positive,
} from "../src/format.ts";
import { History } from "../src/history.ts";

describe("format helpers", () => {
  test("recognise images by extension, case insensitively, and not dotfiles", () => {
    expect(isImage("a/b/Logo.PNG")).toBe(true);
    expect(isImage("icon.svg")).toBe(true);
    expect(isImage("notes.md")).toBe(false);
    expect(isImage(".png")).toBe(false);
    expect(extensionOf("a.tar.gz")).toBe("gz");
  });

  test("print sizes the way the viewer does", () => {
    expect(human(512)).toBe("512 B");
    expect(human(4500)).toBe("4.4 KB");
    expect(human(2 * 1024 * 1024)).toBe("2.0 MB");
    expect(dimensions(64, 32)).toBe("64 × 32");
    expect(change(1000, 500)).toBe("50% smaller");
    expect(change(500, 1000)).toBe("100% larger");
    expect(change(5, 5)).toBe("same size");
  });

  test("keep the ratio when one side changes", () => {
    const from = { width: 200, height: 100 };
    expect(aspectHeight(50, from)).toBe(25);
    expect(aspectWidth(10, from)).toBe(20);
    expect(aspectHeight(1, { width: 1000, height: 1 })).toBe(1);
  });

  test("accept only positive whole pixel counts", () => {
    expect(positive("64")).toBe(64);
    expect(positive("12.6")).toBe(13);
    expect(positive("0")).toBeNull();
    expect(positive("-3")).toBeNull();
    expect(positive("abc")).toBeNull();
  });
});

describe("history", () => {
  const bytes = (n: number) => new Uint8Array([n]);

  test("undoes one step at a time along a rename chain", () => {
    const history = new History();
    history.push({ from: "a.png", bytes: bytes(1), to: "a.png" });
    history.push({ from: "a.png", bytes: bytes(2), to: "a.webp" });

    expect(history.count("a.webp")).toBe(2);
    expect(history.undo("a.webp")).toEqual({ from: "a.png", bytes: bytes(2), to: "a.webp" });
    expect(history.count("a.png")).toBe(1);
    expect(history.undo("a.png")?.bytes).toEqual(bytes(1));
    expect(history.undo("a.png")).toBeNull();
  });

  test("reverts to the very first original in one go", () => {
    const history = new History();
    history.push({ from: "a.png", bytes: bytes(1), to: "a.png" });
    history.push({ from: "a.png", bytes: bytes(2), to: "a.jpg" });

    expect(history.revert("a.jpg")).toEqual({ from: "a.png", bytes: bytes(1), to: "a.jpg" });
    expect(history.count("a.jpg")).toBe(0);
  });
});
