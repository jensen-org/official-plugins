# Jensen official plugins

The source monorepo for the plugins Jensen ships and maintains. Every plugin here is built, released,
and documented in one place.

This is **not** the catalog. The catalog Jensen fetches lives in
[jensen-org/plugin-store](https://github.com/jensen-org/plugin-store), which carries only the index
entries. Source here, catalog there, and the two are joined by a release tag and a checksum.

## Layout

```
plugins/
  hotspots/        ranks services by churn crossed with coupling (TypeScript, uses git + graph)
  common-themes/   thirteen ported palettes (no code at all)
scripts/release.sh builds, publishes, and cuts the GitHub release for one plugin
AUTHORING.md       the full plugin authoring guide
CONTRIBUTING.md    how to add or change a plugin here
```

Plugins are written in **TypeScript** against `@jensen/plugin` and published with `jensen publish`,
which generates the manifest and assembles the release for you. A theme plugin ships no code at all.
Rust-to-WebAssembly is a supported advanced path, documented in `AUTHORING.md`.

## Releases

Every plugin in this monorepo releases from **this repository**, tagged `<plugin>-v<version>`:

```
https://github.com/jensen-org/official-plugins/releases/tag/hotspots-v0.1.0
```

The tag has to carry the plugin name because two plugins cannot both own the tag `0.1.0`. A plugin in
its own repository can keep a plain `0.1.0` tag and omit `jensen.tag` entirely.

Cut a release with:

```sh
scripts/release.sh plugins/hotspots
```

It builds, runs `jensen publish`, uploads every asset in `release/`, and prints the entry to submit to
the store.

## Every plugin needs a README

`jensen publish` refuses a plugin directory without a `README.md`, and CI here rejects one too. The
README is not decoration: it is shipped as a release asset, pinned by the same checksum chain as the
code, and rendered in the app when a user opens the plugin's detail view **before deciding to
install**. It is the only thing standing between a permission prompt and a blind yes.

Say what the plugin does, what capabilities it asks for and why, and what it does when it has nothing
to show.

## Adding a plugin

See `CONTRIBUTING.md`. In short: add a directory under `plugins/`, write the `README.md`, point
`jensen.repo` at this repo with a `<plugin>-v<version>` tag, run `scripts/release.sh`, then open a PR
on the store adding your entry.
