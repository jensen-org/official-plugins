# Images Tools

Resize, compress, round the corners of and convert images right where you open them. Open any image in
Jensen and a toolbar appears above it. Every change shows up at once: the picture, its pixel size, its
file size and, after a conversion, its name and extension.

AI agents get the same tools, so they can prepare images for you without leaving your project.

## What it does

| Tool | What happens |
|---|---|
| Resize | Set a width and a height, with the aspect ratio locked or free. |
| Compress | JPEG at the quality you pick. PNG is recompressed without losing a pixel, and stored as a palette when it has 256 colors or fewer. SVG is minified. A file that cannot get smaller is left alone. |
| Round corners | A radius in pixels, or Circle. The edges are smooth and transparent. A JPEG becomes a PNG, because a JPEG cannot be transparent. |
| Convert | PNG, JPEG, WebP, GIF, BMP, ICO and TIFF. The original is replaced by the new file, and the open tab follows it. |

Every change can be undone with Undo, or all the way back with Revert, for as long as the toolbar stays open.
SVG files are never overwritten: resizing, rounding or converting one draws it sharp as a new raster next to the
original.

You can also right click an image (or several) in the explorer to compress it or convert it to WebP, and run the
same from the command palette.

## Formats

Reads and writes PNG, JPEG, GIF, WebP, BMP, ICO and TIFF, and reads SVG. WebP is written lossless. SVG text is not
drawn when an SVG is turned into a raster, because the plugin ships no fonts. AVIF is not supported.

## For agents

With the plugin enabled, `list_plugin_tools` shows these tools and `call_plugin_tool` runs them:
`dev_jensen_images_tools_info`, `_resize`, `_compress`, `_round_corners` and `_convert`. They work on project
relative paths and only reach the image types you allowed.

## What it asks for, and why

| Permission | Why |
|---|---|
| Files of these types: png, jpg, jpeg, gif, webp, bmp, ico, tif, tiff, svg | to read an image, and write the changed one next to it, anywhere in your project. It cannot touch any other kind of file. |
| Runs its own program | the image work is done by a Rust program bundled with the plugin and run in a sandbox inside Jensen. It has no network and no access beyond the files above. |
| Read your code editor | to know which image you have open, show its toolbar, and move the tab to the new file after a conversion. It does not read or change your code. |

## Build it yourself

```bash
bun install
bun run build          # builds the Rust backend to backend.wasm, then main.js
bun run test           # TypeScript tests, then the Rust tests
bunx jensen-plugin publish
```

The backend is a Rust crate in `backend/` using the `image` and `resvg` crates, compiled to
`wasm32-unknown-unknown`. Add the target with `rustup target add wasm32-unknown-unknown`.
