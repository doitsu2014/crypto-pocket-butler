# core/native — generated from AWS AI-DLC

This directory is a normalised snapshot of **AI-DLC 2.11.0** (tag `v2.11.0`,
commit `6a378b53c0a4fe0641ed7d8de8dfff94264d5b6a`) from [`awslabs/aidlc-workflows`](https://github.com/awslabs/aidlc-workflows), pinned in
[`aidlc-version.json`](../../aidlc-version.json).

**Do not edit anything here by hand.** Every file is owned by `my-aidlc update`
(`node scripts/fetch-native.mjs`), which re-downloads the pinned source, verifies it, and rewrites
this tree.

| Directory | Contents |
| --- | --- |
| `stages/<phase>/` | The 33 native stage definitions, byte-for-byte |
| `agents/` | The 14 native agent personas |
| `scopes/` | The 11 native baseline workflow scopes |
| `protocols/` | The 9 native stage protocols (reference) |
| `phases/` | my-aidlc's display and gate metadata for the five native phases |
| `lock.json` | The pin plus a SHA-256 for every file in this tree |
| `ADAPTATION.md` | How native engine calls map onto the my-aidlc engine |

Stage bodies keep the upstream `/aidlc` and `.claude` placeholders, so the packager
binds them per harness instead of freezing one harness's paths.

Workflow scopes the team authors live in the workspace, not here:
`aidlc/spaces/<space>/scopes/<name>.md`. They layer over the baseline in `scopes/`.
