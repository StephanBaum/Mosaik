# CI: what runs when

A pull request runs only the jobs its changes can affect. Pushes to `main`, the nightly run
(03:00 UTC) and manual runs run everything. Decision record: ADR-0167 in `docs/DECISIONS.md`.

The repository is on the free plan: 20 concurrent jobs, 5 of them macOS. Before ADR-0167, every pull
request started about 55 jobs, and a Windows Rust build takes up to two hours, so a few open pull
requests kept the queue busy for hours.

## How it works

1. `Detect changes` (the `changes` job in `.github/workflows/ci.yml`) runs
   `scripts/ci/changes.mjs`. On a pull request, the script diffs the head against its merge base with
   `github.event.pull_request.base.sha` and maps every changed file to areas. Any other event gives a
   full plan. It needs only Node and git.
2. Each job reads the plan from the `changes` outputs:
   - **Required checks** (the 17 names in the `main` ruleset) always run. When their area is
     untouched, they skip the heavy steps and end green in seconds with a "Not affected" step that
     prints the plan. An unaffected Windows or macOS entry of a matrix runs on `ubuntu-latest`.
     The check name keeps its OS (`Rust (macos-latest)`), so it holds no scarce runner.
   - **Coverage and fuzz** (not required) get their matrix from the plan (`fromJSON`): only the
     affected crates and fuzz directories. An empty list skips the job.
3. Before the plan, the job runs the self-test `node --test scripts/ci/changes.test.mjs`.

### Fail closed

Every required job has `needs: changes` and `if: ${{ !cancelled() }}`, so it also runs when
`changes` failed. Its first step checks `needs.changes.result == 'success'` and
`needs.changes.outputs.ok == 'true'`. If either is missing, the step fails, so the check is red and
the pull request cannot merge. A broken detection never skips anything silently. The self-test runs
inside `changes`, so a broken map fails the same way. The required jobs use `!cancelled()` rather
than `always()`, so a cancelled run (a newer push) does not keep them going. A cancelled check is
not green either.

`mobile.yml` is the exception: there the iOS job runs whenever the detection fails ("run when in
doubt"). It is not a required check.

## Area map

When a pull request touches several areas, the plan is the union of their jobs. "L+W" means
ubuntu-latest and windows-latest.

| Changed path | What runs |
|---|---|
| `.github/**`, `scripts/ci/**`, `.cargo/**`, root `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `deny.toml`, `clippy.toml`, `rustfmt.toml`, `package.json`, `pnpm-lock.yaml`, `pnpm-workspace.yaml`, `.npmrc`, `tsconfig*.json`, `eslint.config.js`, `vitest.config.ts`, `.gitattributes` | **everything** |
| any file the map below does not cover (a new top-level folder, a new crate directory) | **everything** |
| `docs/**`, `marketing/**`, `*.md` outside the code folders, root `LICENSE*` | nothing (the wording check still runs) |
| `.gitignore`, `.editorconfig`, `.prettierignore`, `.vscode/**` | nothing |
| `crates/core/**`, `crates/vault/**`, `vendor/**`, `tools/**` | all Rust on L+W: Rust, CLI, desktop Rust, server image, every coverage and fuzz job |
| `crates/<x>/**` | Rust on L+W for `x` and every crate that builds on it (`-p` each). The CLI, desktop Rust and the server image run if they are among those crates. Coverage and fuzz run for the affected crates |
| `crates/platform/**`, `crates/native-messaging/**`, `crates/cli/src/**`, `apps/desktop/src-tauri/**` | the same, plus **macOS** (keychain, Touch ID, native-messaging and CLI paths that only build there) |
| `crates/<x>/fuzz/**` | that fuzz job, plus fmt and the unsafe check (Rust, ubuntu-latest) |
| `apps/desktop/src-tauri/**` | desktop Rust (L+W+macOS), the desktop fuzz job, fmt and the unsafe check |
| `apps/desktop/**` (the frontend) | TypeScript (3 OS), desktop frontend |
| `apps/extension/**` | TypeScript (3 OS), extension |
| `apps/web/**` | TypeScript (3 OS), `web` (the website job; it uses `needs.changes.outputs.web`) |
| `packages/**` | TypeScript (3 OS), desktop frontend, extension, web; `packages/i18n` and `packages/design-tokens` also iOS |
| `apps/mobile/**` | TypeScript on ubuntu-latest (the mobile export drift check); `apps/mobile/ios/**` also iOS (`mobile.yml`) |
| `deploy/**`, `.dockerignore` | server image |
| any `Cargo.toml` or `Cargo.lock` (fuzz crates included) | also Rust supply chain and the release self-test |
| any `package.json`, `scripts/release/**` | also the release self-test |
| any `.js`, `.mjs`, `.cjs`, `.ts`, `.tsx` (not in `apps/mobile` or `docs/design`) | also TypeScript (3 OS): ESLint lints every script |
| `scripts/mobile/**` | `mobile.yml` only; the non-Android scripts also run iOS |
| `scripts/preview/**` | `preview.yml` only |

The Rust area does not include `crates/core` and `crates/vault` in the macOS rule. A pure core
change runs on Linux and Windows. `main` and the nightly run then cover macOS.

### Dependencies

The script reads the path dependencies of every workspace manifest (`crates/*`,
`apps/desktop/src-tauri`) and every fuzz manifest as text. This matches `cargo metadata`. A crate
is affected when its normal dependencies, followed transitively, reach a changed crate. It is also
affected when one of its dev-dependencies is (or builds on) a changed crate, because its tests use
them. Dev-dependencies do not count transitively. The server image needs normal dependencies only. An
internal crate declared without `path` (for example `mosaic-x.workspace = true`) stops the script, so
the graph cannot silently miss an edge. A change to the workspace members list runs everything.

### Files that other areas' tests read

Some tests read files of another area. The map routes those files to the reading job:

| File | Read by |
|---|---|
| `apps/extension/keys/**` | `mosaic-native-messaging` tests |
| `apps/desktop/src-tauri/wix/**` | `mosaic-native-messaging` tests |
| `apps/desktop/src-tauri/tauri*.conf.json` | `mosaic-platform` tests, the desktop frontend build |
| `apps/desktop/src/ipc/contract.fixtures.ts` | desktop Rust (contract fixture check) |
| `apps/desktop/src/components/icons/**` | the extension (popup icons) |
| `packages/i18n/src/catalog/*/insights.ts` | `mosaic-insights` tests |
| `packages/i18n/src/catalog/*/{sync,sharing}.ts` | desktop Rust |
| `crates/server/assets/**`, `crates/passkey/fixtures/**`, `crates/native-messaging/fixtures/**` | the extension and TypeScript (vitest, e2e) |

The self-test finds every relative path in a tracked `.rs`, `.ts`, `.tsx` or `.mjs` file under
`crates/`, `apps/` and `packages/` that points into another area. It fails when the map does not
route that target to the referring job. So a new cross-area reference fails `Detect changes` in the
pull request that adds it, until the map is updated.

## What always runs

- **On every pull request**: `Detect changes`, the 17 required checks (as no-ops where unaffected),
  and the banned-claims wording check in `TypeScript (ubuntu-latest)`. That check scans `README.md`,
  `SECURITY.md` and parts of `docs/`.
- **On `main`, the nightly schedule and `workflow_dispatch`**: everything, on every OS. That is all
  14 coverage jobs, all 16 fuzz jobs, the supply chain checks (new advisories show up here even when
  no lockfile changed), the release self-test and iOS. CodeQL is set up in the repository settings,
  not in these workflows.

## Forcing a full run

- **`workflow_dispatch`**: Actions → CI → "Run workflow" on the branch. The checks attach to the
  branch head, which is the pull request's head commit.
- **Label `ci:full`**: add it to the pull request, then push or re-run the workflow. `Detect changes`
  reads the current labels through the API (the event's labels if the API fails) and plans a full
  run. Adding the label alone starts nothing: the workflow does not listen to `labeled` events, so
  unrelated labels never restart CI.

## Concurrency

A new push to a pull request cancels its previous run. Runs on `main`, the schedule and manual runs
are never cancelled. While one runs, the next waits. GitHub keeps only the newest waiting run per
group, so a burst of merges ends with the latest commit tested. `mobile.yml` does the same.

## Adding a job

A new **required** job follows the pattern of `server-image`:

```yaml
  web:
    name: Website (...)
    needs: changes
    if: ${{ !cancelled() }}
    runs-on: ubuntu-latest
    env:
      RUN: ${{ needs.changes.outputs.web == 'true' }}
    steps:
      - name: Change detection gate (fails closed)
        shell: bash
        env:
          RESULT: ${{ needs.changes.result }}
          OK: ${{ needs.changes.outputs.ok }}
        run: |
          if [ "$RESULT" != success ] || [ "$OK" != true ]; then
            echo "::error::Change detection did not succeed (result: $RESULT). Failing closed (docs/CI.md)."
            exit 1
          fi
      - if: env.RUN == 'true'
        uses: actions/checkout@... # every heavy step carries the same `if`
      - name: Not affected
        if: env.RUN != 'true'
        env:
          SUMMARY: ${{ needs.changes.outputs.summary }}
        run: echo "No-op. Plan, $SUMMARY"
```

Add its area to `classify()` in `scripts/ci/changes.mjs` and an output to the `changes` job. Add a
case to `scripts/ci/changes.test.mjs`, and a row to the area map above. The `web` output exists
already. Never put `if:` on a required job itself, and never rename a required job or its matrix:
the ruleset matches the check names exactly.

## Checking a diff locally

```sh
git diff --name-only --no-renames origin/main...HEAD > /tmp/files.txt
node scripts/ci/changes.mjs --files /tmp/files.txt
node --test scripts/ci/changes.test.mjs
```
