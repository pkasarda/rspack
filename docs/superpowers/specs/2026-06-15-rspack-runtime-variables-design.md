# Rspack Runtime Variables Design

## Context

Rspack already has `experiments.runtimeMode` with two supported values:

- `webpack`
- `rspack`

The RFC at `/Users/bytedance/Documents/团队/rspack/RFC-Rspack-Runtime-Variables.md` defines rspack-style names for runtime internals that Rspack generates in bundle output. The current implementation has many runtime rendering paths wired through `RuntimeTemplate`, but the root-scope runtime variable names are still mostly rendered with webpack-style names.

## Goals

When `experiments.runtimeMode` is `rspack`, Rspack-generated root-scope runtime internals should use rspack-style names:

| webpack-style | rspack-style |
| --- | --- |
| `__webpack_require__` | `__rspack_require` |
| `__webpack_modules__` | `__rspack_modules` |
| `__webpack_module_cache__` | `__rspack_module_cache` |
| `__webpack_exports__` | `__rspack_exports` |
| `__webpack_module__` | `__rspack_module` |
| `__webpack_exec__` | `__rspack_exec` |
| `__rspack_context` | `__rspack_context` |

When `experiments.runtimeMode` is `webpack`, generated output must keep the existing webpack-style names.

Generated comments are observable bundle output for this change. Comments that mention these generated internals should follow the same mode-aware names as the code around them.

## Non-Goals

This change does not update parser recognition for module variables in user source code. User-authored `__webpack_require__`, `__webpack_modules__`, and related module variables continue to be recognized as they are today. User-authored `__rspack_*` module variables are not introduced in this change.

This change does not add a new option. It only completes behavior behind the existing `experiments.runtimeMode`.

This change does not rename non-output implementation comments, test fixture source code, or compatibility parser documentation unless they are part of generated output snapshots being asserted for this feature.

## Architecture

Use a centralized mode-aware mapping for `RuntimeVariable` rendering.

The preferred path is:

1. `CompilerOptions.experiments.runtime_mode`
2. `RuntimeTemplate`
3. `runtime_variable_to_string(runtime_variable, compiler_options)`
4. Generated runtime, chunk, and module code

`runtime_variable_name(...)` can remain the static webpack-style/default helper, but `runtime_variable_to_string(...)` should become the source of truth for output rendering when compiler options are available.

Most root-scope render sites already call `runtime_template.render_runtime_variable(...)`, so updating the central mapping should update code and nearby generated comments together. Remaining hard-coded generated strings should be changed to use the same helper.

`RuntimeGlobals` rendering must stay compatible with the existing runtime-context split:

- In webpack mode, `RuntimeGlobals::REQUIRE_SCOPE`, `RuntimeGlobals::REQUIRE`, and related require-scope globals keep webpack-style rendering.
- In rspack module rendering, module code should continue to use `__rspack_context`, `__rspack_context.r`, and `__rspack_context.<property>` where the existing runtime-context architecture requires it.
- In rspack runtime/chunk rendering, root-scope internals such as require, modules, module cache, exports, module, and startup exec should render with rspack-style names.

## Expected Implementation Areas

Primary files:

- `crates/rspack_core/src/runtime_globals.rs`
- `crates/rspack_core/src/runtime_template.rs`

Likely follow-up scan areas:

- `crates/rspack_plugin_javascript/src/plugin/mod.rs`
- `crates/rspack_plugin_javascript/src/plugin/runtime_context.rs`
- `crates/rspack_plugin_esm_library/src/render.rs`
- `crates/rspack_plugin_runtime/src/*chunk_format.rs`
- Runtime modules that format `RuntimeVariable` names into generated code

The scan should distinguish generated output strings from Rust comments or parser compatibility comments. Generated output strings should be mode-aware. Internal comments may stay webpack-style when they describe compatibility behavior or parser behavior.

## Testing

Add or extend focused runtime-mode tests.

For `runtimeMode: "rspack"`, assert representative output contains rspack-style root internals such as:

- `__rspack_require`
- `__rspack_modules`
- `__rspack_module_cache`
- `__rspack_exports`
- `__rspack_exec` when startup exec is emitted

Also assert generated comments use rspack-style names, for example:

- `// expose the modules object (__rspack_modules)`

And assert the same output does not contain generated webpack-style root internals such as:

- `__webpack_require__`
- `__webpack_modules__`
- `__webpack_module_cache__`
- `__webpack_exports__`

For webpack mode, add or keep a regression check that default output continues to use webpack-style names.

The parser/module-variable tests should not be rewritten to expect `__rspack_*` user-source recognition.

## Verification

Run focused runtime-mode test cases after implementation.

Because this change touches Rust runtime rendering, run a Rust binding build or equivalent project build before completion.

Expected verification commands:

- `pnpm run build:binding:dev`
- A focused `tests/rspack-test` command for the runtime-mode cases changed by this work

## Risks

The main risk is accidentally changing webpack mode output. Centralizing the mapping should reduce this risk, but webpack-mode tests should verify it.

The second risk is missing hard-coded generated strings, especially comments or EJS/runtime snippets. A repository scan for the old runtime internal names in relevant generated output snapshots should be part of implementation.

The third risk is overreaching into parser behavior. This design explicitly excludes parser recognition of `__rspack_*` module variables.
