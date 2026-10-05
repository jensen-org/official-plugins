# Building Jensen plugins

The authoring guide lives with the SDK, so it is versioned with the API it describes:

- Docs: <https://github.com/jensen-org/plugin-sdk/tree/main/docs>
- Start a plugin: `npm create jensen-plugin@latest my-plugin`
- API: `jensen-plugin-sdk` (the `Plugin` class), `jensen-ui` (the panes' building blocks)

The rules that matter when you add a plugin here:

- Plugins run in a sandbox with no ambient authority. They reach files, the editor, the layout, the
  theme, the code graph, git and the network only through calls Jensen checks against the permissions
  the user granted.
- No call exposes a stored secret, keychain item or credential.
- Jensen has three pages and a plugin cannot add one. A plugin adds panes.
- Ask for the least you can: every entry in `jensen.permissions` is a line on the consent screen.
- Every plugin needs a `README.md` that says what it does and why it asks for each permission.
