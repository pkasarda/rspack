# Rspack runtime variables implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Rspack-generated root-scope runtime internals render with rspack-style names when `experiments.runtimeMode` is `"rspack"` while preserving webpack mode and parser behavior.

**Architecture:** Centralize the mode-aware `RuntimeVariable` mapping in `rspack_core`, then let existing `RuntimeTemplate` call sites pick up the new names. Keep module runtime-context rendering intact: module bodies may still use `__rspack_context`, while root runtime variables become `__rspack_require`, `__rspack_modules`, `__rspack_module_cache`, `__rspack_exports`, `__rspack_module`, and `__rspack_exec` in rspack mode. Tests assert both code and generated comments.

**Tech Stack:** Rust, Rspack runtime rendering, `rstest`, `tests/rspack-test` config cases.

---

## File structure

- Modify `crates/rspack_core/src/runtime_globals.rs`
  - Owns `RuntimeVariable` names.
  - Add the rspack-mode mapping here.

- Modify `crates/rspack_core/src/runtime_template.rs`
  - Owns `RuntimeGlobalsRenderMap`.
  - Replace direct webpack-style `runtime_variable_name(...)` calls in render-map construction where runtime/chunk mode should use rspack-style root names.

- Modify `tests/rspack-test/configCases/runtime/runtime-mode-module-rendering/test.config.js`
  - Main focused rspack-mode assertion for root variable names and generated comments.

- Modify `tests/rspack-test/configCases/runtime/runtime-mode-async-chunk/test.config.js`
  - Update expectations where runtime context points at the root require function.

- Modify `tests/rspack-test/configCases/runtime/runtime-mode-require-context/test.config.js`
  - Add a narrow regression that `require.context` remains separate from `__rspack_context` while root require is rspack-style.

- Snapshot updates are not expected for this work. Treat snapshot churn as a signal to inspect the generated output before changing snapshots.

Do not modify parser module-variable logic in `crates/rspack_plugin_javascript/src/parser_plugin/api_plugin.rs`. This PR intentionally keeps user-source recognition limited to `__webpack_*`.

### Task 1: write failing Runtime-Mode tests

**Files:**

- Modify: `tests/rspack-test/configCases/runtime/runtime-mode-module-rendering/test.config.js`
- Modify: `tests/rspack-test/configCases/runtime/runtime-mode-async-chunk/test.config.js`
- Modify: `tests/rspack-test/configCases/runtime/runtime-mode-require-context/test.config.js`

- [ ] **Step 1: Update `runtime-mode-module-rendering` assertions**

Replace `tests/rspack-test/configCases/runtime/runtime-mode-module-rendering/test.config.js` with:

```js
const fs = require('fs');
const path = require('path');

/** @type {import("../../../..").TConfigCaseConfig} */
module.exports = {
  afterExecute(options) {
    const source = fs.readFileSync(
      path.resolve(options.output.path, 'main.js'),
      'utf-8',
    );

    expect(source).toContain('var __rspack_context={};');
    expect(source).toContain('__rspack_context.d');
    expect(source).toContain('__rspack_context.N');
    expect(source).toContain('__rspack_context.d = definePropertyGetters;');
    expect(source).toContain('__rspack_context.N = makeNamespaceObject;');
    expect(source).toContain('module.exports, __rspack_context');
    expect(source).toContain('definePropertyGetters =');
    expect(source).toContain('makeNamespaceObject =');

    expect(source).toMatch(/function __rspack_require\s*\(\s*moduleId\s*\)/);
    expect(source).toMatch(/var __rspack_module_cache\s*=\s*\{\};/);
    expect(source).toMatch(/var __rspack_exports\s*=/);
    expect(source).toContain('// The module cache');
    expect(source).toContain('// The require function');
    expect(source).toContain('// expose the modules object (__rspack_modules)');

    expect(source).not.toContain('__webpack_require__');
    expect(source).not.toContain('__webpack_modules__');
    expect(source).not.toContain('__webpack_module_cache__');
    expect(source).not.toContain('__webpack_exports__');
    expect(source).not.toContain(
      '// expose the modules object (__webpack_modules__)',
    );
    expect(source).not.toContain('__webpack_require__.d(__webpack_exports__');
    expect(source).not.toContain('__webpack_require__.r(__webpack_exports__');
    expect(source).not.toContain('__webpack_require__.d =');
    expect(source).not.toContain('__webpack_require__.r =');
  },
};
```

- [ ] **Step 2: Update `runtime-mode-async-chunk` root require expectation**

In `tests/rspack-test/configCases/runtime/runtime-mode-async-chunk/test.config.js`, change the main-source root require assertion to:

```js
expect(mainSource).toContain('__rspack_context.r = __rspack_require;');
expect(mainSource).toMatch(/function __rspack_require\s*\(\s*moduleId\s*\)/);
expect(mainSource).toMatch(/var __rspack_module_cache\s*=\s*\{\};/);
expect(mainSource).not.toContain('__webpack_require__');
expect(mainSource).not.toContain('__webpack_module_cache__');
```

Keep the existing async chunk assertions:

```js
expect(asyncChunkSource).toContain('__rspack_context.d');
expect(asyncChunkSource).not.toContain('__rspack_install_runtime');
expect(asyncChunkSource).not.toContain('__webpack_require__.d');
```

- [ ] **Step 3: Add root require checks to `runtime-mode-require-context`**

In `tests/rspack-test/configCases/runtime/runtime-mode-require-context/test.config.js`, keep the existing assertions and add:

```js
expect(source).toMatch(/function __rspack_require\s*\(\s*moduleId\s*\)/);
expect(source).toContain('__rspack_context.r = __rspack_require;');
expect(source).not.toContain('__webpack_require__');
```

- [ ] **Step 4: Run focused tests and verify they fail for the intended reason**

Run:

```bash
PATH="/Users/bytedance/.cache/codex-tools/pnpm-11.6.0/bin:/Users/bytedance/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin:$PWD/node_modules/.bin:$PATH" pnpm --dir tests/rspack-test run test:base -- -t runtime/runtime-mode-module-rendering
```

Expected: FAIL because generated output still contains `__webpack_require__` or does not contain `__rspack_require`.

Run:

```bash
PATH="/Users/bytedance/.cache/codex-tools/pnpm-11.6.0/bin:/Users/bytedance/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin:$PWD/node_modules/.bin:$PATH" pnpm --dir tests/rspack-test run test:base -- -t runtime/runtime-mode-async-chunk
```

Expected: FAIL because generated output still contains `__rspack_context.r = __webpack_require__;`.

- [ ] **Step 5: Commit failing tests**

```bash
git add tests/rspack-test/configCases/runtime/runtime-mode-module-rendering/test.config.js tests/rspack-test/configCases/runtime/runtime-mode-async-chunk/test.config.js tests/rspack-test/configCases/runtime/runtime-mode-require-context/test.config.js
PATH="/Users/bytedance/.cache/codex-tools/pnpm-11.6.0/bin:/Users/bytedance/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin:$PWD/node_modules/.bin:$PATH" git commit -m "test: cover rspack runtime variable names"
```

### Task 2: implement Mode-Aware RuntimeVariable names

**Files:**

- Modify: `crates/rspack_core/src/runtime_globals.rs`
- Modify: `crates/rspack_core/src/runtime_template.rs`

- [ ] **Step 1: Update imports in `runtime_globals.rs`**

Replace the current single import near the top of `crates/rspack_core/src/runtime_globals.rs`:

```rust
use crate::CompilerOptions;
```

with:

```rust
use crate::{CompilerOptions, runtime_mode::RuntimeMode};
```

- [ ] **Step 2: Replace `runtime_variable_to_string`**

Replace the existing implementation in `crates/rspack_core/src/runtime_globals.rs` with:

```rust
pub fn runtime_variable_to_string(
  runtime_variable: &RuntimeVariable,
  compiler_options: &CompilerOptions,
) -> String {
  match compiler_options.experiments.runtime_mode {
    RuntimeMode::Webpack => runtime_variable_name(runtime_variable).to_string(),
    RuntimeMode::Rspack => rspack_runtime_variable_name(runtime_variable).to_string(),
  }
}
```

- [ ] **Step 3: Add `rspack_runtime_variable_name`**

Add this function directly below `runtime_variable_to_string`:

```rust
pub fn rspack_runtime_variable_name(runtime_variable: &RuntimeVariable) -> &'static str {
  match *runtime_variable {
    RuntimeVariable::Require => "__rspack_require",
    RuntimeVariable::Context => "__rspack_context",
    RuntimeVariable::Modules => "__rspack_modules",
    RuntimeVariable::ModuleCache => "__rspack_module_cache",
    RuntimeVariable::Exports => "__rspack_exports",
    RuntimeVariable::Module => "__rspack_module",
    RuntimeVariable::StartupExec => "__rspack_exec",
  }
}
```

Do not change `runtime_variable_name`; it remains the webpack-style static helper.

- [ ] **Step 4: Import the new helper in `runtime_template.rs`**

In `crates/rspack_core/src/runtime_template.rs`, update the `runtime_globals::{ ... }` import to include `rspack_runtime_variable_name`:

```rust
  runtime_globals::{
    RuntimeVariable, rspack_runtime_variable_name, runtime_globals_to_string, runtime_variable_name,
    runtime_variable_to_string,
  },
```

Use the repository formatter later if line wrapping differs.

- [ ] **Step 5: Update `runtime_globals_to_render_map` for rspack runtime/chunk mode**

In the `RuntimeGlobalRenderMode::RspackRuntimeModule` match arm in `crates/rspack_core/src/runtime_template.rs`, change the root require and module factory branches from webpack-style helper calls to rspack-style helper calls:

```rust
      RuntimeGlobalRenderMode::RspackRuntimeModule => {
        if runtime_globals == RuntimeGlobals::REQUIRE_SCOPE {
          runtime_variable_name(&RuntimeVariable::Context).to_string()
        } else if runtime_globals == RuntimeGlobals::REQUIRE {
          rspack_runtime_variable_name(&RuntimeVariable::Require).to_string()
        } else if runtime_globals == RuntimeGlobals::MODULE_FACTORIES {
          rspack_runtime_variable_name(&RuntimeVariable::Modules).to_string()
        } else if runtime_globals.renderable_require_scope() == runtime_globals {
          runtime_globals.to_lexical_name().map_or_else(
            || runtime_globals_to_string(&runtime_globals),
            str::to_string,
          )
        } else {
          runtime_globals_to_string(&runtime_globals)
        }
      }
```

Leave the `RspackModule` branch using `__rspack_context` because module rendering should keep runtime-context access.

- [ ] **Step 6: Format Rust files**

Run:

```bash
PATH="/Users/bytedance/.cache/codex-tools/pnpm-11.6.0/bin:/Users/bytedance/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin:$PWD/node_modules/.bin:$PATH" pnpm run format:rs
```

Expected: Rust formatting completes successfully.

- [ ] **Step 7: Run focused tests from Task 1**

Run:

```bash
PATH="/Users/bytedance/.cache/codex-tools/pnpm-11.6.0/bin:/Users/bytedance/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin:$PWD/node_modules/.bin:$PATH" pnpm --dir tests/rspack-test run test:base -- -t runtime/runtime-mode-module-rendering
```

Expected: PASS.

Run:

```bash
PATH="/Users/bytedance/.cache/codex-tools/pnpm-11.6.0/bin:/Users/bytedance/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin:$PWD/node_modules/.bin:$PATH" pnpm --dir tests/rspack-test run test:base -- -t runtime/runtime-mode-async-chunk
```

Expected: PASS.

- [ ] **Step 8: Commit implementation**

```bash
git add crates/rspack_core/src/runtime_globals.rs crates/rspack_core/src/runtime_template.rs
PATH="/Users/bytedance/.cache/codex-tools/pnpm-11.6.0/bin:/Users/bytedance/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin:$PWD/node_modules/.bin:$PATH" git commit -m "fix(runtime): render rspack runtime variables"
```

### Task 3: scan generated output strings and adjust remaining tests

**Files:**

- Inspect: `crates/rspack_plugin_javascript/src/plugin/mod.rs`
- Inspect: `crates/rspack_plugin_javascript/src/plugin/runtime_context.rs`
- Inspect: `crates/rspack_plugin_esm_library/src/render.rs`
- Inspect: `crates/rspack_plugin_runtime/src/*chunk_format.rs`
- Modify tests only when assertions still expect generated webpack-style names in rspack mode.

- [ ] **Step 1: Scan for generated hard-coded runtime internals**

Run:

```bash
rg -n "__webpack_(require|modules|module_cache|exports|module|exec)__|expose the modules object \\(__webpack_modules__\\)" crates/rspack_core crates/rspack_plugin_javascript crates/rspack_plugin_runtime crates/rspack_plugin_esm_library crates/rspack_plugin_library crates/rspack_plugin_rstest -g '!**/tests/**'
```

Expected: remaining hits are either parser compatibility, implementation comments, or generated strings that intentionally stay webpack mode. Generated output strings in rspack-mode paths should either already use `render_runtime_variable(...)` or be changed to do so.

- [ ] **Step 2: Keep rstest mock runtime API migration out of scope**

This file injects mock runtime code that is coupled to rstest runtime APIs and parser-recognized webpack module variables. Do not broaden this PR into rstest runtime API migration.

- [ ] **Step 3: Run all runtime-mode config tests**

Run:

```bash
PATH="/Users/bytedance/.cache/codex-tools/pnpm-11.6.0/bin:/Users/bytedance/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin:$PWD/node_modules/.bin:$PATH" pnpm --dir tests/rspack-test run test:base -- -t RuntimeModeConfig.part1
```

Expected: PASS.

Run:

```bash
PATH="/Users/bytedance/.cache/codex-tools/pnpm-11.6.0/bin:/Users/bytedance/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin:$PWD/node_modules/.bin:$PATH" pnpm --dir tests/rspack-test run test:base -- -t RuntimeModeConfig.part2
```

Expected: PASS.

Run:

```bash
PATH="/Users/bytedance/.cache/codex-tools/pnpm-11.6.0/bin:/Users/bytedance/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin:$PWD/node_modules/.bin:$PATH" pnpm --dir tests/rspack-test run test:base -- -t RuntimeModeConfig.part3
```

Expected: PASS.

- [ ] **Step 4: Update only failing assertions that are about generated output names**

If a failure comes from expected generated output such as:

```js
expect(source).toContain('__rspack_context.r = __webpack_require__;');
```

change it to:

```js
expect(source).toContain('__rspack_context.r = __rspack_require;');
```

If a failure comes from parser module-variable behavior, keep existing behavior and do not change parser logic.

- [ ] **Step 5: Commit scan/test adjustments**

```bash
git add tests/rspack-test crates/rspack_core crates/rspack_plugin_javascript crates/rspack_plugin_runtime crates/rspack_plugin_esm_library crates/rspack_plugin_library crates/rspack_plugin_rstest
PATH="/Users/bytedance/.cache/codex-tools/pnpm-11.6.0/bin:/Users/bytedance/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin:$PWD/node_modules/.bin:$PATH" git commit -m "test(runtime): update rspack mode expectations"
```

If there are no changes after the scan, skip this commit.

### Task 4: final verification

**Files:**

- No planned file edits.

- [ ] **Step 1: Build Rust binding**

Run:

```bash
PATH="/Users/bytedance/.cache/codex-tools/pnpm-11.6.0/bin:/Users/bytedance/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin:$PWD/node_modules/.bin:$PATH" pnpm run build:binding:dev
```

Expected: PASS.

- [ ] **Step 2: Run focused runtime-mode tests again**

Run:

```bash
PATH="/Users/bytedance/.cache/codex-tools/pnpm-11.6.0/bin:/Users/bytedance/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin:$PWD/node_modules/.bin:$PATH" pnpm --dir tests/rspack-test run test:base -- -t runtime/runtime-mode-module-rendering
```

Expected: PASS.

Run:

```bash
PATH="/Users/bytedance/.cache/codex-tools/pnpm-11.6.0/bin:/Users/bytedance/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin:$PWD/node_modules/.bin:$PATH" pnpm --dir tests/rspack-test run test:base -- -t runtime/runtime-mode-async-chunk
```

Expected: PASS.

Run:

```bash
PATH="/Users/bytedance/.cache/codex-tools/pnpm-11.6.0/bin:/Users/bytedance/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin:$PWD/node_modules/.bin:$PATH" pnpm --dir tests/rspack-test run test:base -- -t runtime/runtime-mode-require-context
```

Expected: PASS.

- [ ] **Step 3: Confirm webpack mode still uses webpack-style variables**

Run:

```bash
PATH="/Users/bytedance/.cache/codex-tools/pnpm-11.6.0/bin:/Users/bytedance/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin:$PWD/node_modules/.bin:$PATH" pnpm --dir tests/rspack-test run test:base -- -t configCases/runtime/runtime-mode-option
```

Expected: existing webpack/default-mode coverage still passes. If this command does not select the default-mode case in this repository, run one small existing config case that does not set `experiments.runtimeMode` and inspect its generated `main.js` for `function __webpack_require__(`.

- [ ] **Step 4: Check status**

Run:

```bash
git status --short --branch
```

Expected: clean working tree, on the implementation branch, ahead by the implementation commits.

- [ ] **Step 5: Summarize results**

Report:

- Files changed.
- Focused tests run and their PASS/FAIL status.
- Whether `pnpm run build:binding:dev` passed.
- Any skipped broader tests.
