---
title: "GitHub Actions"
---

The official [`happyview-actions`](https://github.com/gamesgamesgamesgamesgames/happyview-actions) repository provides GitHub Actions that deploy your lexicons, scripts, jobs, and labelers to a HappyView instance straight from a Git repository, allowing you to keep your instance in sync with version-controlled source.

There are two actions:

| Action                                                  | Purpose                                                                                                                       |
| ------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| `gamesgamesgamesgamesgames/happyview-actions/deploy@v1` | Uploads lexicons and scripts to a HappyView instance via the [admin API](../api-reference/admin/admin-api.md).                |
| `gamesgamesgamesgamesgames/happyview-actions/lint@v1`   | Offline validation of the same files (no network, no secrets) — run it on pull requests to catch problems before they deploy. |

Both are convention-driven: what gets deployed, and the trigger each script binds to, is derived entirely from your directory layout.

## Prerequisites

You need an [API key](api-keys.md) with permission to write the resources you deploy. Create one under **Settings > API Keys** in the dashboard with these scopes:

- `lexicons:create` — upload lexicons
- `scripts:manage` — upload route, record, job, and labeler scripts

If you enable [pruning](#pruning), the key also needs `lexicons:read`, `lexicons:delete`, and `scripts:read` so the action can list and remove remote items that no longer exist in your git repo.

Store the key and instance URL as repository secrets (e.g. `HAPPYVIEW_API_KEY` and `HAPPYVIEW_URL`). Never hardcode the key in the workflow file.

## Repository layout

Files are discovered by directory, using these defaults:

| Category             | Default pattern                           | Deploys as                                                                               |
| -------------------- | ----------------------------------------- | ---------------------------------------------------------------------------------------- |
| Lexicons             | `lexicons/**/*.json`                      | Uploaded lexicons (NSID read from the `id` field)                                        |
| Route/record scripts | `lua/**/*.lua` (path mirrors the lexicon) | `xrpc.query` / `xrpc.procedure` / `record.index`, depending on the paired lexicon's type |
| Job scripts          | `jobs/**/*.lua`                           | `job.run:<type>`                                                                         |
| Labeler scripts      | `labelers/**/*.lua`                       | `labeler.apply:<nsid>` (or `labeler.apply:_actor`)                                       |

A route or record script is **paired** to a lexicon when their paths match (ignoring the base directory and file extension). For example, `lua/app/feed/getTimeline.lua` pairs with `lexicons/app/feed/getTimeline.json`; if that lexicon is a `query`, the script deploys under the trigger `xrpc.query:app.feed.getTimeline`. See [record scripts](record-scripts.md), [background jobs](background-jobs.md), and [labelers](labelers.md) for what each trigger does.

Job and labeler triggers come from the file path relative to their directory, with `/` replaced by `.`:

```
jobs/nightly/rebuild-cache.lua     ->  job.run:nightly.rebuild-cache
labelers/app.bsky.feed.post.lua    ->  labeler.apply:app.bsky.feed.post
labelers/_actor.lua                ->  labeler.apply:_actor
```

<Callout type="info">
Each of the `lexicons`, `scripts`, `jobs`, and `labelers` inputs is a newline-delimited list of glob patterns, so you can point them at any layout. A pattern beginning with `!` excludes matches — e.g. list `lua/**/*.lua` and `!lua/jobs/**` to keep job scripts out of the route/record set when they live under `lua/`.
</Callout>

## Quickstart

This workflow lints every pull request and deploys on merges to `main`:

```yaml
name: Deploy to HappyView

on:
  pull_request:
  push:
    branches: [main]

jobs:
  lint:
    if: github.event_name == 'pull_request'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: gamesgamesgamesgamesgames/happyview-actions/lint@v1

  deploy:
    if: github.event_name == 'push'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: gamesgamesgamesgamesgames/happyview-actions/deploy@v1
        with:
          happyview-url: ${{ vars.HAPPYVIEW_URL }}
          api-key: ${{ secrets.HAPPYVIEW_API_KEY }}
```

## `deploy` inputs

| Input           | Required | Default              | Description                                                                                                    |
| --------------- | -------- | -------------------- | -------------------------------------------------------------------------------------------------------------- |
| `happyview-url` | Yes      | —                    | Base URL of your HappyView instance.                                                                           |
| `api-key`       | Yes      | —                    | An `hv_` [API key](api-keys.md). Always pass this from a secret.                                               |
| `admin-path`    | No       | `/admin`             | Mount path of the admin API. Set this if your instance is served under a base path (for example, `/hv/admin`). |
| `lexicons`      | No       | `lexicons/**/*.json` | Glob pattern list for lexicon JSON files.                                                                      |
| `scripts`       | No       | `lua/**/*.lua`       | Glob pattern list for route/record scripts.                                                                    |
| `jobs`          | No       | `jobs/**/*.lua`      | Glob pattern list for job scripts.                                                                             |
| `labelers`      | No       | `labelers/**/*.lua`  | Glob pattern list for labeler scripts.                                                                         |
| `backfill`      | No       | `true`               | Sets the [backfill](backfill.md) flag when uploading record lexicons.                                          |
| `prune`         | No       | `false`              | Delete remote lexicons/scripts that no longer exist in the repo. See [Pruning](#pruning).                      |
| `changed-files` | No       | —                    | Limit the deploy to a set of changed paths. See [Deploying only what changed](#deploying-only-what-changed).   |
| `dry-run`       | No       | `false`              | Log everything the action _would_ do without making any changes.                                               |

The action does not fail on the first error. Instead, it attempts every file, reports each result in the job summary, and exits non-zero if anything failed. Uploads are idempotent upserts, so re-running a deploy is always safe.

## `lint` inputs

The `lint` action takes the same `lexicons`, `scripts`, `jobs`, and `labelers` glob inputs, plus:

| Input              | Default | Description                                                  |
| ------------------ | ------- | ------------------------------------------------------------ |
| `check-lua-syntax` | `true`  | Parse every discovered `.lua` file and report syntax errors. |
| `fail-on`          | `error` | Fail the action on `error`, or on `warning` to be stricter.  |

It reports orphaned scripts (a `.lua` with no matching lexicon), invalid NSIDs and job types, duplicate triggers, and Lua parse errors as inline annotations.

## Deploying only what changed

On a large repository you can skip unchanged files by passing a list of changed paths to `changed-files` (a JSON array or a newline-delimited list). A common pattern is to compute it with [`dorny/paths-filter`](https://github.com/dorny/paths-filter) and forward the result:

```yaml
- uses: gamesgamesgamesgamesgames/happyview-actions/deploy@v1
  with:
    happyview-url: ${{ vars.HAPPYVIEW_URL }}
    api-key: ${{ secrets.HAPPYVIEW_API_KEY }}
    changed-files: ${{ steps.filter.outputs.changed_files }}
```

When a lexicon or its paired script changes, both are re-deployed together. Leaving `changed-files` empty deploys everything.

## Pruning

By default the action only creates and updates, but never deletes. Set `prune: true` to make the repository the single source of truth: any lexicon or script on the instance whose trigger isn't produced by the current repo scan is deleted.

<Callout type="warn">
Pruning is destructive and operates on the whole instance. A misconfigured path pattern can delete lexicons and scripts you meant to keep. Combine it with `dry-run: true` first to preview exactly what would be removed. Pruning always uses the full repository scan as its baseline, so it is safe to combine with `changed-files` — the incremental filter limits uploads, never deletions.
</Callout>

## Marketplace and versioning

The actions are consumed by path reference (`owner/happyview-actions/deploy@v1`), like other subdirectory actions. Pin to the major tag `@v1` to receive non-breaking updates automatically, or pin to an exact release tag if you prefer to upgrade deliberately.
