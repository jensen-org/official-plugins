# Common Themes

Thirteen ported palettes, packaged as one installable theme plugin. These shipped with the app until
the built-in set was curated down from twenty-three to ten; installing this brings the rest back.

## Palettes

| Dark | Light |
|---|---|
| Dracula Alucard | Gruvbox Light |
| Gruvbox Dark | Kanagawa Lotus |
| Kanagawa Dragon | One Light |
| Kanagawa Wave | Solarized Light |
| One Dark | Tokyo Night Light |
| Rosé Pine Moon | |
| Solarized Dark | |
| Tokyo Night Storm | |

## Using it

Install, then pick any of them from Settings, Appearance, Theme. They appear alongside the built-in
themes with no restart. Removing the plugin removes its themes; if one was active, the app falls back
to the default.

## Permissions

**None.** This plugin ships no code at all: no `main.js`, no WebAssembly module, and no activation
event. It is a set of colour documents plus a manifest that declares them, so there is nothing to
sandbox and nothing to consent to.

That is worth knowing when you audit it: a theme plugin cannot read your files, reach the network, or
run on startup, because it has no executable surface to do so from.

## Adding a palette

Each file under `themes/` is a theme document validated at publish time against the app's theme
schema, so a malformed palette fails the publish rather than the install. To add one, drop the
document in `themes/` and add an `{ id, name, file }` entry to `jensen.contributes.themes` in
`package.json`. There is no build step.
