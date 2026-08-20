# Contributing a plugin

This repository holds the plugins Jensen maintains. If you are publishing your own plugin from your
own repository, you do not need to change anything here: build it, release it, and open a PR on
[jensen-org/plugin-store](https://github.com/jensen-org/plugin-store) with your entry.

Read `AUTHORING.md` first. It covers the security model, the `Plugin` class, capabilities, and the
declarative UI.

## Adding a plugin to this monorepo

1. **Create `plugins/<name>/`** with a `package.json` carrying identity in its `jensen` block:

   ```json
   "jensen": {
     "id": "dev.jensen.<name>",
     "category": "graph",
     "minAppVersion": "0.0.0",
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
   screen. `fs` takes relative subpaths and `network` takes exact hosts; neither accepts a wildcard.

4. **Build and release:**

   ```sh
   scripts/release.sh plugins/<name>
   ```

5. **Submit the entry** it prints to the store repository.

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

Building and testing plugins in CI is wired up but disabled, because `@jensen/plugin` is still a
`file:` path dependency on a local app checkout. It turns on when the SDK is published to npm.
