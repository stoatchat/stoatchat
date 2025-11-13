# Plugin API

:::danger
This page documents the old Revite Plugin API (manifest v1), it will be replaced in the new client.
:::

:::warning
The Plugin API is very powerful. **Tread carefully.**

**Zero guarantees or sandboxes are provided.** Your code is run as-is.
:::

This document details the very experimental plugin API available in [Revite](https://github.com/revoltchat/revite).

This is more or less a proof of concept but can be used to achieve some simple client modifications.

## Plugin Manifest

Below is the specification for revision 1 of the plugin API. The `format` parameter is not currently enforced but you should set it to `1` to avoid future breakage.

````typescript
type Plugin = {
  /**
   * Plugin Format Revision
   */
  format: 1;

  /**
   * Semver Version String
   *
   * This is the version of the plugin.
   */
  version: string;

  /**
   * Plugin Namespace
   *
   * This will usually be the author's name.
   */
  namespace: string;

  /**
   * Plugin Id
   *
   * This should be a valid URL slug, e.g. cool-plugin.
   */
  id: string;

  /**
   * Entrypoint
   *
   * Valid Javascript code. It must be a function which returns a object.
   *
   * ```typescript
   * function (state: State) {
   *   return {
   *     onUnload: () => {}
   *   }
   * }
   * ```
   */
  entrypoint: string;

  /**
   * Whether this plugin is enabled.
   *
   * @default true
   */
  enabled?: boolean;
};
````

An example plugin:

```javascript
{
    format: 1,
    version: "0.0.1",
    namespace: "insert",
    id: "my-plugin",
    entrypoint: `(state) => {
        console.log('[my-plugin] Plugin init!');
        return {
            onUnload: () => console.log('[my-plugin] bye!')
        }
    }`
}
```

## Using the Plugin API

To begin, you can load plugins using the global plugin manager at `state.plugins`.

Open the developer console and run:

```javascript
state.plugins.load({ ... });
// ...where [...] is your plugin manifest as described above.
```

## Plugin API

A plugin's entrypoint is required to return an object which is referred to as the **instance**:

```typescript
interface Instance {
  onUnload?: () => void;
}
```

The Plugin API (`state.plugins`) exposes the following methods:

```typescript
interface PluginAPI {
  /**
   * Add a plugin
   * @param plugin Plugin Manifest
   */
  add(plugin: Plugin);

  /**
   * Remove a plugin
   * @param namespace Plugin Namespace
   * @param id Plugin Id
   */
  remove(namespace: string, id: string);

  /**
   * Load a plugin
   * @param namespace Plugin Namespace
   * @param id Plugin Id
   */
  load(namespace: string, id: string);

  /**
   * Unload a plugin
   * @param namespace Plugin Namespace
   * @param id Plugin Id
   */
  unload(namespace: string, id: string);

  /**
   * Reset everything
   */
  reset();
}
```
