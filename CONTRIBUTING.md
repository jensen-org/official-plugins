# Contributing a plugin

This repository holds the plugins Jensen maintains. If you are publishing your own plugin from your
own repository, you do not need to change anything here: build it, release it, and open a PR on
[jensen-org/plugins-store](https://github.com/jensen-org/plugins-store) with your entry.

Read `AUTHORING.md` first. It points at the SDK docs, which cover the security model, the `Plugin`
class, permissions and panes.

## Adding a plugin to this monorepo

1. **Create `plugins/<name>/`** with a `package.json` carrying identity in its `jensen` block:

   ```json
   "jensen": {
     "id": "dev.jensen.<name>",
     "category": "graph",
     "minAppVersion": "0.3.0",
     "repo": "jensen-org/official-plugins",
     "tag": "<name>-v0.1.0",
     "permissions": { "graph": true }
   }
   ```

   `repo` and `tag` are both required here: releases come from this repository, so the tag has to
   name the plugin. CI checks that `tag` equals `<name>-v<version>`.

2. **Write `README.md`.** Required, and enforced by both `jensen publish` and CI. It ships as a
   release asset and is what a user reads in the app before installing.

3. **Ask for the least you can.** Every capability in `permissions` becomes a line on the consent
   screen. `fs` takes a folder (`"docs"`), a file type (`"*.png"`), or `"."` for every file, so name
   the narrowest scope that works. `network` takes exact hosts and no wildcard.

4. **Open a pull request against `develop`.** The maintainer reviews and approves it, then promotes
   `develop` into `main`. Nothing ships until it is on `main`.

5. **Build and release from `main`:**

   ```sh
   scripts/release.sh plugins/<name>
   ```

6. **Submit the entry** it prints to the store repository.

## Changing an existing plugin

Bump `version` in `package.json` **and** `jensen.tag` to match, since CI pins them together. Release
as above, then open a store PR updating that plugin's entry with the new version, tag and `sha256`.

An unchanged `sha256` with a bumped version means you shipped the old manifest; the store's CI
verifies the checksum against the live release and will reject it.

## What gets checked

CI on this repository verifies, for every plugin, that it has a README, that `jensen.repo` points
here, that `jensen.tag` follows the convention, and that it declares an id. The store's CI does the
other half: schema validation, that the release actually exists, and that its manifest checksum
matches the entry.

CI also builds and tests every plugin that has a `build` script, including the Rust backend of plugins
that ship one, so the runner needs the `wasm32-unknown-unknown` target.
