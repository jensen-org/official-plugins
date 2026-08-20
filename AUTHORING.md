# Building Jensen plugins

Jensen is moddable. A plugin can add buttons, commands, whole views, panels, status items, and
themes, and it can read the project's code graph, query the knowledge index, inspect git history, and
read or write files, all without ever touching your machine directly.

You write plugins in **TypeScript**, the way you would an Obsidian plugin: a `Plugin` class with an
`onload()` method that registers what you contribute. There is no toolchain to install beyond Node,
and no manifest to hand-write; `jensen publish` generates it for you.

That last part, the not-touching-your-machine part, is the whole design. Jensen runs as a **kernel**:
it is the only actor that ever touches the filesystem, the network, git, or the OS. Your plugin runs
in a sandbox with zero ambient authority. It cannot open a file, a socket, or a process, cannot reach
the app's window, and cannot see or talk to another plugin. The only way it causes any effect is to
ask the kernel to perform an action on its behalf, a brokered call the kernel checks against the
permissions you declared and the user consented to, meters against a resource budget, performs itself,
and can deny or revoke at any time. Deny by default.

## Table of contents

- [The security model](#the-security-model)
- [Quickstart](#quickstart)
- [The Plugin class](#the-plugin-class)
- [Capabilities](#capabilities)
- [Adding a UI](#adding-a-ui)
- [package.json and the jensen block](#packagejson-and-the-jensen-block)
- [Publishing](#publishing)
- [Resource limits](#resource-limits)
- [The advanced path: Rust to WebAssembly](#the-advanced-path-rust-to-webassembly)
- [Security checklist](#security-checklist)

## The security model

Your plugin has two halves, and one broker sits between them and the app:

- **Logic** runs in a null-origin sandboxed iframe served from the `plugin://` origin. It is
  cross-origin to the app (so it cannot read the app's window, storage, or another plugin) and its
  Content-Security-Policy sets `connect-src 'none'`, so it has **no network of its own**. Its only way
  out is a brokered call, `jensen.<capability>`, relayed to the kernel.
- **UI** (optional) runs in its own null-origin iframe. It has no kernel access at all; it talks only
  to your logic half, by posting messages the host relays.
- **Every effect** flows through the kernel: `charge the resource budget → check the capability →
  perform the action → return a result`. A denied call returns an error, it never crashes the app, and
  the user is shown which capability was blocked so they can grant it or keep it blocked.

Chrome you add (buttons, commands, views, panels, status items) is rendered by Jensen's own UI from a
plain description you register; your code never runs in the app's window. When your plugin is disabled,
every registration is reclaimed and its sandbox is torn down.

## Quickstart

Prerequisites: Node and a bundler (esbuild is one line). A plugin is a small TypeScript project.

```
my-plugin/
  package.json         identity + the jensen block (permissions, category, repo)
  src/main.ts          your Plugin class
  main.js              the bundled output your build produces
  ui/                  optional iframe UI (index.html, ...)
```

`src/main.ts`:

```ts
import { Plugin } from "@jensen/plugin";

export default class extends Plugin {
  async onload() {
    this.addRibbonIcon("cloud", "Scan cluster", () => this.scan());

    this.addCommand({
      id: "gcp.scan",
      name: "GCP: Scan cluster",
      callback: () => this.scan(),
    });

    this.registerView({ id: "gcp", label: "GCP", icon: "cloud" });
  }

  async scan() {
    const impact = await this.graph.impact({ path: "infra/main.tf" });
    await this.log.info(`scan touched ${JSON.stringify(impact)}`);
  }
}
```

`package.json`:

```json
{
  "name": "GCP Scanner",
  "version": "1.0.0",
  "description": "Scan clusters from the graph.",
  "author": "Your Name",
  "scripts": {
    "build": "esbuild src/main.ts --bundle --format=esm --outfile=main.js"
  },
  "devDependencies": { "@jensen/plugin": "*", "esbuild": "*" },
  "jensen": {
    "id": "dev.you.gcp-scanner",
    "category": "cloud",
    "permissions": { "graph": true }
  }
}
```

Build, then load it for local testing:

```bash
npm run build
```

`jensen publish` (below) turns this into a manifest, a release folder, and a registry entry. For local
iteration, copy the contents of that `release/` folder into `~/.local/share/jensen/plugins/<id>/`
(unzipping `ui.zip` into `ui/` if you have one), restart Jensen, open the **Plugins** page, and enable
the plugin. Your button appears in the top bar, your command in the palette, your view in the switcher.
Disable it and all of that disappears in one tick.

## The Plugin class

Extend `Plugin` and default-export it. Override `onload` (and optionally `onunload`); everything you
register is disposed automatically when the plugin unloads.

| Method | Adds |
| --- | --- |
| `addRibbonIcon(icon, title, callback)` | A button in the top bar. `icon` is a lucide name. |
| `addCommand({ id, name, callback })` | A command in the palette (and keymap). |
| `registerView({ id, label, icon })` | A top-level view (an entry in the header switcher). |
| `addStatusBarItem({ id, label?, icon?, tooltip?, onClick? })` | A status-bar item. |
| `register(disposable)` | Track any `{ dispose() }` for automatic cleanup on unload. |

Each `addX` returns a disposable, so you can remove a contribution before unload if you want. `icon`
values are [lucide](https://lucide.dev) names (`"cloud"`, `"git-branch"`, `"sparkles"`), drawn from a
fixed allowlist. An unknown name is a validation error that fails the publish, not a fallback glyph.

Persist your own JSON with `this.loadData()` / `this.saveData(obj)`; it is stored in your plugin's
private data directory and needs no permission.

```ts
async onload() {
  const state = (await this.loadData<{ runs: number }>()) ?? { runs: 0 };
  state.runs++;
  await this.saveData(state);
}
```

## Capabilities

Reach the kernel through `this.<capability>`. Every call is brokered and rejects if the capability is
not granted. Request each in the `jensen.permissions` block (below).

```ts
await this.graph.impact({ target: "src/auth.rs" });    // needs permissions.graph
await this.graph.dependencies({ target: "src/auth.rs", incoming: true });
await this.graph.query({ /* ... */ });
await this.graph.explainService({ /* ... */ });
await this.knowledge.search({ query: "how login works" }); // needs permissions.knowledge
await this.git.history({ path: "src/auth.rs", limit: 20 }); // needs permissions.git
await this.git.semanticDiff({ /* ... */ });
await this.fs.read("docs/spec.md");                    // needs a matching permissions.fs scope
await this.fs.write("docs/out.md", contents);
await this.net.fetch({ host: "api.example.com", path: "/v1/status" }); // needs the host in permissions.network
await this.log.info("hello");
```

`net.fetch` takes `{ host, path?, method? }`. The kernel performs the HTTPS request itself: `host` must
be one you listed (no wildcards, no ports), the connection is pinned to the host's resolved public
address so a redirect cannot escape it, and private and loopback addresses are refused.

## Adding a UI

Describe the surface and let Jensen draw it. A page or pane returns `ui` nodes; the host renders them
with its own components, so your surface follows the user's theme, needs no HTML or CSS, and has no
message plumbing to get wrong.

```ts
import { Plugin, ui } from "@jensen/plugin";

export default class extends Plugin {
  private rows: { service: string; commits: number }[] = [];

  async onload() {
    this.addPage({
      id: "hotspots",
      label: "Hotspots",
      icon: "workflow",
      render: () => [
        ui.row([ui.heading("Hotspots"), ui.button({ label: "Rescan", onClick: () => this.scan() })], {
          align: "between",
        }),
        ui.table({
          columns: [
            { key: "service", label: "Service", align: "start" },
            { key: "commits", label: "Commits" },
          ],
          rows: this.rows,
          empty: "Nothing ranked yet.",
        }),
      ],
    });
  }

  async scan() {
    this.rows = await gather();
    this.refresh();          // redraw every surface this plugin owns
  }
}
```

`addPane` is the same call for a right-drawer tab. A click runs your closure in the plugin sandbox and
the surface redraws, so the whole model is: change state, call `refresh()`.

Available nodes: `heading, text, code, badge, divider, stack, row, section, table, list, keyValue,
stat, progress, button, empty, spinner, message`. Nodes a host is too old to know are skipped rather
than failing the surface.

### The escape hatch: your own HTML

For rich views, ship an `ui/` folder. It runs in a second null-origin iframe with no kernel access; it
talks only to your logic half through a typed bridge. Use `@jensen/plugin-ui`:

```js
import { PluginUI } from "@jensen/plugin-ui";
const jensen = new PluginUI();

document.getElementById("run").addEventListener("click", async () => {
  const result = await jensen.invokeCommand("gcp.scan", { region: "eu" });
  document.getElementById("out").textContent = JSON.stringify(result);
});
```

In your logic half, handle those calls (return a value to resolve the promise) and push events with the
UI event channel. The UI iframe has no network and cannot reach the app; the only path in or out is
this bridge.

Call `await jensen.ready()` first. It returns `{ pluginId, theme }` and applies the app's design
tokens to your document, so your CSS can use the same `var(--fg)`, `var(--surface)` and `var(--sp-3)`
the app does, and it keeps applying them when the user changes theme. Skipping it leaves your surface
with whatever palette you hardcoded, which will drift from the user's theme: your iframe is sandboxed
to a null origin, so it cannot read the app's custom properties on its own.

```js
const { theme } = await jensen.ready();   // tokens are on :root from here on
document.body.style.background = "var(--bg-0)";
```

## package.json and the jensen block

Everything the manifest needs is derived from `package.json`. Standard npm fields supply `name`,
`version`, `description`, and `author`. The `jensen` block supplies what cannot be inferred:

| `jensen.` field | Meaning |
| --- | --- |
| `id` | Reverse-DNS identity, lowercase letters, digits, `.` and `-`. Optional; derived from `name` if omitted. |
| `category` | Free-form grouping shown on the card and used by the category filter. |
| `minAppVersion` | Lowest Jensen version you support. Defaults to the app you publish from. |
| `repo` | `owner/name` of the GitHub repo hosting your releases. |
| `tag` | Release tag, when it is not just `version`. A monorepo shipping several plugins needs one tag per plugin, by convention `<plugin>-v<version>`. Omit it and the tag is the version. |
| `permissions` | `{ graph, knowledge, git, fs, network }`. See below. Deny by default. |
| `contributes` | Optional. Declare views here to make them appear before your code runs (lazy load). Most plugins register imperatively in `onload` instead and leave this out. |

`permissions`:

- `graph` / `knowledge` / `git`: booleans, gate the matching capabilities.
- `fs`: array of **relative** subpaths (e.g. `["docs"]`). Each confines file access to
  `<project root>/<subpath>`. `..`, absolute paths, and backslashes are rejected.
- `network`: array of **exact** hosts (e.g. `["api.example.com"]`).

What you write is only what you *request*. The kernel enforces the **granted** set, your request
intersected with what the user approved on the consent screen. A user can withhold, say, network, and
those calls return a permission error, so degrade rather than crash.

## Publishing

Publishing has no form. From your built plugin directory:

```bash
jensen publish            # or: jensen publish path/to/plugin
```

It reads `package.json`, finds what your build produced, generates `manifest.json` (activating on
startup so your `onload` runs), assembles a `release/` folder holding the manifest, your `README.md`,
and every artifact it pins, computes the manifest's `sha256`, and prints the store entry to add:

```json
{
  "id": "dev.you.gcp-scanner",
  "name": "GCP Scanner",
  "author": "Your Name",
  "description": "Scan clusters from the graph.",
  "category": "cloud",
  "version": "1.0.0",
  "min_app_version": "0.1.0",
  "repo": "you/gcp-scanner",
  "sha256": "…"
}
```

**A `README.md` is required.** `jensen publish` fails without one. It ships as a release asset pinned
by the same checksum as your code, and Jensen renders it in the plugin's detail view *before* the user
installs, so it is what they read while deciding whether to grant your permissions. Cover what the
plugin does, which capabilities it asks for and why, and how it behaves when a permission is withheld.

You can also run it from the app: **Plugins → Publish**, pick your folder, and copy the entry.

Then:

1. Create a GitHub release on your `repo` whose tag equals `version`, uploading every file in
   `release/`. If you set `jensen.tag`, use that instead.
2. To reach every user, open a pull request on
   [jensen-org/plugin-store](https://github.com/jensen-org/plugin-store) adding the entry above as
   `entries/<id>.json`. Its CI validates the entry, fetches your release, and checks the checksum
   before the catalog is rebuilt, so a broken entry fails on your PR rather than in someone's app.
   To test or share privately, users can install the release URL directly; Jensen marks such plugins
   **unverified** and asks for the full permission consent.

### How installation works

The store's `index.json` lists, per plugin, `{ id, name, author, description, category, version,
min_app_version, repo, sha256 }` plus an optional `tag`. It locates a plugin (`repo` + tag) and pins it
(`sha256` is the checksum of the release `manifest.json`); it carries no capabilities and no code. To
install, Jensen
downloads the release `manifest.json`, checks it against `sha256`, validates it, confirms its id, and
stages the plugin disabled until the user approves its permissions. There are no keys to manage: what a
plugin may do is enforced by the sandbox and the consent screen, not the registry.

## Resource limits

Every plugin runs under a resource governor. The defaults per plugin:

| Limit | Default | Meaning |
| --- | --- | --- |
| Syscall rate | 50 / s | A token bucket; bursts beyond it are rejected. |
| Concurrency | 4 | At most this many brokered calls in flight at once. |
| I/O budget | 256 MiB | Cumulative bytes across fs and network before calls are refused. |

A runaway plugin is stopped and the user is told why; the user can also force-stop any plugin. Keep
per-command work bounded and handle a denied or rate-limited call gracefully.

## The advanced path: Rust to WebAssembly

TypeScript is the default. For CPU-heavy or existing-Rust logic you can instead ship a WebAssembly
module built against the `jensen-plugin` PDK; it runs in the same kernel with the same permissions and
governor.

Nothing about publishing changes. `package.json` stays the only file you author, and `jensen publish`
picks up a `plugin.wasm` at the project root, or the newest module under
`target/wasm32-unknown-unknown/release/`, the same way it picks up `main.js`. A `ui/` folder is zipped
and pinned for you. Themes need no code at all: ship a theme document conforming to
`schema/theme.schema.json` and declare a `contributes.themes` entry whose `file` names it. One plugin
may contribute several, as `common-themes` does with thirteen.

A plugin with code activates on startup by default. Set `jensen.activationEvents` yourself to load
lazily instead (`["onCommand:hotspots.scan", "onPanel:hotspots.panel"]`), or to `[]` for something
like a theme, which contributes no code to run.

## Security checklist

- Request the **least** you need. Every capability you declare is shown to the user and raises your
  risk score. A plugin that asks for nothing installs with no friction.
- Scope `fs` to the narrowest subpath that works, never the whole project.
- List **exact** network hosts. There are no wildcards, and the kernel will not follow a redirect to
  any host you did not list.
- Keep per-command work bounded so you stay inside the rate and I/O budgets.
- Never assume a call succeeds. A user can withhold any capability, and your plugin should degrade,
  not crash.
