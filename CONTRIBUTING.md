# Contributing content

Read the [org-wide rules](https://github.com/rustly-tech/.github/blob/main/CONTRIBUTING.md)
first. This file covers what is specific to content.

## Before you open a PR

```sh
cd tools && cargo build --release && cd ..
T=./tools/target/release/rustly-content
$T validate && $T check-examples && $T check-trials && $T schemas --check
```

All four must pass. CI runs the same commands plus an upstream attribution check
that needs network access.

## Writing a lesson

- **Short.** A lesson is a single idea a reader can hold in their head. If it has
  three sections that could each stand alone, it is three lessons.
- **Show the error.** Include the real diagnostic, and teach the reader to read
  all of it rather than only the first line.
- **Original prose.** Cite sources in `references`; do not paste from them.
- Every example carries an `expect`, so CI can prove it still does what you said.
  A deliberately broken example is valuable *because* it is deliberate.

## Writing a quiz

- Exactly one correct option.
- Every wrong option needs `why_wrong`. A plausible wrong answer is a specific
  misconception, and naming it is where the teaching happens.
- The `explanation` is shown whether the reader was right or wrong, so write it
  for both.

## Adding an OSS example

Required, and enforced: `repository`, a full 40-character lowercase `commit`,
`path`, `line_start`, `line_end`, the upstream `license`, and `attribution`.

Pin the commit, never a branch:

```sh
gh api repos/<owner>/<name>/commits/main --jq .sha
```

Choose an excerpt short enough to learn from — CI warns past 60 lines — and add a
`simplified` rewrite when the original needs context a learner does not have yet.
Check the upstream license permits the excerpt and record it exactly.

## Adding a Trial

1. `content/trials/<slug>/` with `trial.json`, `statement.md`, `starter.rs`,
   `solution.rs`.
2. `starter_expect` must describe what the starter really does. If it is meant to
   fail with `E0382`, CI checks that it fails with `E0382`.
3. At least one public test. Add hidden tests too: without them a solution tuned
   to the visible cases passes, and CI will warn you.
4. Hidden tests should probe what the public ones do not — multi-byte input,
   repeated whitespace, boundary sizes.
5. `version` starts at 1 and increases whenever the tests or the statement change
   meaningfully. A verdict is only meaningful against a specific version.

## Editorial lifecycle

`draft` → `beta` → `verified` → `official`

- `draft` is not published and is not indexed.
- `beta` is published with a marker; solves do not count toward ranking.
- `verified` has been reviewed; solves count.
- `official` is curated by maintainers as canonical for a concept.

New community Trials start at `draft`.

## Licensing your contribution

Content contributions are CC BY 4.0. Tooling contributions are MIT OR Apache-2.0.
By opening a PR you agree to license your work accordingly.
