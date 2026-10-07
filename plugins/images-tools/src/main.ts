import { type FileStat, Notice, Plugin, type UiNode, ui } from "@jensen-org/plugin-sdk";
import {
  aspectHeight,
  aspectWidth,
  change,
  dimensions,
  extensionOf,
  human,
  isImage,
  positive,
  TARGETS,
} from "./format.ts";
import { History } from "./history.ts";

interface Info {
  path: string;
  format: string;
  width: number;
  height: number;
  bytes: number;
}

interface Report extends Info {
  before: Info;
  removed: string | null;
  unchanged: boolean;
}

interface Form {
  width: string;
  height: string;
  lock: boolean;
  quality: string;
  radius: string;
  target: string;
  busy: boolean;
  note: { text: string; tone: "info" | "success" | "danger" } | null;
}

const TOOLBAR = "images";

export default class ImagesTools extends Plugin {
  private readonly history = new History();
  private readonly infos = new Map<string, { signature: string; info: Info }>();
  private shownFor: string | null = null;
  private noteFor: string | null = null;
  private form: Form = {
    width: "",
    height: "",
    lock: true,
    quality: "80",
    radius: "12",
    target: "webp",
    busy: false,
    note: null,
  };

  onload() {
    this.registerViewerToolbar(TOOLBAR, { kinds: ["image"] }, ({ path }) => this.render(path));
    this.registerCommands();
    this.registerExplorerMenu();
  }

  private async info(path: string): Promise<Info> {
    const stat: FileStat | null = await this.app.files.stat(path);
    const signature = `${stat?.size}:${stat?.modifiedMs}`;
    const cached = this.infos.get(path);
    if (cached && cached.signature === signature) return cached.info;
    const info = await this.app.backend.call<Info>("info", { path });
    this.infos.set(path, { signature, info });
    return info;
  }

  private async render(path: string): Promise<UiNode[]> {
    let info: Info;
    try {
      info = await this.info(path);
    } catch (cause) {
      return [ui.Message(cause instanceof Error ? cause.message : String(cause), "danger")];
    }
    if (this.shownFor !== path) {
      this.shownFor = path;
      this.form.width = String(info.width);
      this.form.height = String(info.height);
      if (this.noteFor !== path) this.form.note = null;
    }
    const busy = this.form.busy;
    const vector = info.format === "svg";

    const resize: UiNode[] = [
      ui.Text("Resize", "muted"),
      ui.Input({
        label: "Width",
        value: this.form.width,
        onChange: (value) => {
          this.form.width = value;
          const width = positive(value);
          if (this.form.lock && width) this.form.height = String(aspectHeight(width, info));
        },
      }),
      ui.Checkbox({
        label: "Lock",
        value: this.form.lock,
        onChange: (value) => {
          this.form.lock = value;
        },
      }),
      ui.Input({
        label: "Height",
        value: this.form.height,
        onChange: (value) => {
          this.form.height = value;
          const height = positive(value);
          if (this.form.lock && height) this.form.width = String(aspectWidth(height, info));
        },
      }),
      ui.Button({
        label: "Resize",
        onClick: () => {
          const width = positive(this.form.width);
          const height = positive(this.form.height);
          if (!width || !height) return this.say("Enter a width and a height in pixels", "danger");
          return this.apply(path, "resize", { width, height, keepAspect: false });
        },
      }),
    ];

    const compress: UiNode[] = [
      ui.Text("Compress", "muted"),
      ...(vector
        ? []
        : [
            ui.Input({
              label: "Quality",
              value: this.form.quality,
              onChange: (value) => {
                this.form.quality = value;
              },
            }),
          ]),
      ui.Button({
        label: "Compress",
        onClick: () =>
          this.apply(path, "compress", { quality: Math.min(100, positive(this.form.quality) ?? 80) }),
      }),
    ];

    const round: UiNode[] = [
      ui.Text("Round corners", "muted"),
      ui.Input({
        label: "Radius px",
        value: this.form.radius,
        onChange: (value) => {
          this.form.radius = value;
        },
      }),
      ui.Button({
        label: "Round",
        onClick: () =>
          this.apply(path, "round_corners", {
            radius: Number(this.form.radius) || 0,
            ...replacing(vector),
          }),
      }),
      ui.Button({
        label: "Circle",
        onClick: () =>
          this.apply(path, "round_corners", { radius: 50, percent: true, ...replacing(vector) }),
      }),
    ];

    const convert: UiNode[] = [
      ui.Text("Convert", "muted"),
      ui.Select({
        label: "To",
        value: this.form.target,
        options: TARGETS.filter((target) => target.value !== extensionOf(path)).map((target) => ({
          value: target.value,
          label: target.label,
        })),
        onChange: (value) => {
          this.form.target = value;
        },
      }),
      ui.Button({
        label: "Convert",
        onClick: () => this.apply(path, "convert", { to: this.form.target }),
      }),
    ];

    const undo: UiNode[] = [
      ui.Button({ label: "Undo", onClick: () => this.undo(path), tone: "muted" }),
      ui.Button({ label: "Revert", onClick: () => this.revert(path), tone: "muted" }),
    ];

    const note = this.form.note;
    return [
      ui.Row(
        [
          ...(busy ? [ui.Spinner("Working")] : []),
          ui.Row(resize, { gap: 6, align: "center" }),
          ui.Row(compress, { gap: 6, align: "center" }),
          ui.Row(round, { gap: 6, align: "center" }),
          ui.Row(convert, { gap: 6, align: "center" }),
          ...(this.history.count(path) > 0 ? [ui.Row(undo, { gap: 6, align: "center" })] : []),
          ...(note ? [ui.Text(note.text, note.tone === "danger" ? "danger" : "muted")] : []),
        ],
        { gap: 16, align: "center" },
      ),
    ];
  }

  private say(
    text: string,
    tone: "info" | "success" | "danger" = "info",
    forPath: string | null = this.shownFor,
  ): void {
    this.form.note = { text, tone };
    this.noteFor = forPath;
    void this.app.viewer.refresh(TOOLBAR);
  }

  private async apply(path: string, method: string, input: Record<string, unknown>): Promise<void> {
    if (this.form.busy) return;
    this.form.busy = true;
    this.say("Working", "info", path);
    try {
      const original = await this.app.files.readBytes(path);
      const { result } = await this.app.backend.run<Report>(method, { path, ...input });
      if (result.unchanged) {
        this.say("Already as small as it gets", "info");
        return;
      }
      this.history.push({ from: path, bytes: original, to: result.path });
      this.shownFor = null;
      if (result.path !== path) await this.app.editor.retarget(path, result.path);
      this.say(summary(method, result), "success", result.path);
    } catch (cause) {
      this.say(cause instanceof Error ? cause.message : String(cause), "danger");
    } finally {
      this.form.busy = false;
      void this.app.viewer.refresh(TOOLBAR);
    }
  }

  private async restore(path: string, from: string, bytes: Uint8Array): Promise<void> {
    await this.app.files.writeBytes(from, bytes);
    if (path !== from) {
      await this.app.editor.retarget(path, from);
      await this.app.files.delete(path);
    }
    this.shownFor = null;
  }

  private async undo(path: string): Promise<void> {
    const step = this.history.undo(path);
    if (!step) return;
    try {
      await this.restore(path, step.from, step.bytes);
      this.say("Undone", "success", step.from);
    } catch (cause) {
      this.say(cause instanceof Error ? cause.message : String(cause), "danger");
    }
  }

  private async revert(path: string): Promise<void> {
    const step = this.history.revert(path);
    if (!step) return;
    try {
      await this.restore(path, step.from, step.bytes);
      this.say("Back to the original", "success", step.from);
    } catch (cause) {
      this.say(cause instanceof Error ? cause.message : String(cause), "danger");
    }
  }

  private async onActive(method: string, input: Record<string, unknown>): Promise<void> {
    const path = this.app.viewer.active?.path;
    if (!path || !isImage(path)) {
      new Notice("Open an image first", { severity: "warn" });
      return;
    }
    await this.apply(path, method, input);
  }

  private registerCommands(): void {
    this.addCommand({
      id: "compress",
      name: "Compress the open image",
      category: "Images",
      icon: "image-down",
      callback: () => this.onActive("compress", {}),
    });
    for (const target of ["webp", "png", "jpg"]) {
      this.addCommand({
        id: `convert-${target}`,
        name: `Convert the open image to ${target.toUpperCase()}`,
        category: "Images",
        icon: "image",
        callback: () => this.onActive("convert", { to: target }),
      });
    }
  }

  private registerExplorerMenu(): void {
    this.registerMenu("explorer", (menu, context) => {
      const images = context.targets.filter(isImage);
      if (images.length === 0 || context.isDirectory) return;
      const label = images.length > 1 ? `${images.length} images` : "image";
      menu.addItem((item) =>
        item
          .setTitle(`Compress ${label}`)
          .setIcon("image-down")
          .onClick(() => this.batch(images, "compress", {})),
      );
      menu.addItem((item) =>
        item
          .setTitle(`Convert ${label} to WebP`)
          .setIcon("image")
          .onClick(() => this.batch(images, "convert", { to: "webp" })),
      );
    });
  }

  private async batch(paths: string[], method: string, input: Record<string, unknown>) {
    let saved = 0;
    let failed = 0;
    for (const path of paths) {
      try {
        const { result } = await this.app.backend.run<Report>(method, { path, ...input });
        if (!result.unchanged) saved += result.before.bytes - result.bytes;
      } catch {
        failed += 1;
      }
    }
    new Notice(
      failed > 0
        ? `${paths.length - failed} of ${paths.length} images done, ${failed} failed`
        : `${paths.length} image${paths.length === 1 ? "" : "s"} done, ${human(Math.abs(saved))} ${saved >= 0 ? "saved" : "added"}`,
      { severity: failed > 0 ? "warn" : "success" },
    );
  }
}

function replacing(vector: boolean): { keepOriginal?: false } {
  return vector ? {} : { keepOriginal: false };
}

function summary(method: string, result: Report): string {
  const before = result.before;
  const size = `${human(before.bytes)} to ${human(result.bytes)}, ${change(before.bytes, result.bytes)}`;
  switch (method) {
    case "resize":
      return `Resized ${dimensions(before.width, before.height)} to ${dimensions(result.width, result.height)}, ${size}`;
    case "convert":
      return `Converted ${before.format} to ${result.format}, ${size}`;
    case "round_corners":
      return `Rounded the corners, ${size}`;
    default:
      return `Compressed ${size}`;
  }
}
