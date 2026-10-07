export const IMAGE_EXTENSIONS = [
  "png",
  "jpg",
  "jpeg",
  "gif",
  "webp",
  "bmp",
  "ico",
  "tif",
  "tiff",
  "svg",
] as const;

export const TARGETS = [
  { value: "png", label: "PNG" },
  { value: "jpg", label: "JPEG" },
  { value: "webp", label: "WebP" },
  { value: "gif", label: "GIF" },
  { value: "bmp", label: "BMP" },
  { value: "ico", label: "ICO" },
  { value: "tiff", label: "TIFF" },
] as const;

export function extensionOf(path: string): string {
  const name = path.split("/").filter(Boolean).at(-1) ?? "";
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(dot + 1).toLowerCase() : "";
}

export function isImage(path: string): boolean {
  return (IMAGE_EXTENSIONS as readonly string[]).includes(extensionOf(path));
}

export function human(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB"];
  let value = bytes;
  let unit = -1;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value >= 10 ? Math.round(value) : value.toFixed(1)} ${units[unit]}`;
}

export function dimensions(width: number, height: number): string {
  return `${width} × ${height}`;
}

export function change(before: number, after: number): string {
  if (after === before) return "same size";
  const percent = Math.round((Math.abs(after - before) / Math.max(before, 1)) * 100);
  return `${percent}% ${after < before ? "smaller" : "larger"}`;
}

export function aspectHeight(width: number, from: { width: number; height: number }): number {
  return Math.max(1, Math.round((width * from.height) / Math.max(from.width, 1)));
}

export function aspectWidth(height: number, from: { width: number; height: number }): number {
  return Math.max(1, Math.round((height * from.width) / Math.max(from.height, 1)));
}

export function positive(text: string): number | null {
  const value = Number(text);
  return Number.isFinite(value) && value >= 1 ? Math.round(value) : null;
}
