# rustly-tech/content

Versioned educational content for [Rustly](https://rustly.tech): learning paths,
lessons, quizzes, cheatsheets, Trials, and curated open-source examples.

Content is **data**, not code. It is validated, compiled, executed, and packaged
by CI before anything reaches the platform.

## What CI proves before a change lands

Content that merely parses is not content that teaches. Every one of these runs
on every pull request:

| Check | What would otherwise slip through |
| --- | --- |
| Schema validation | A field renamed in one document and not another |
| Identifier and reference checks | A lesson pointing at a quiz that was renamed |
| **Committed schemas match the Rust types** | Documentation that disagrees with the validator |
| **Every example does what it claims** | A broken-on-purpose example that broke differently |
| **Starters behave as declared** | A starter that already compiles, removing the exercise |
| **Reference solutions pass every test** | A Trial nobody can solve, including its author |
| **Emitted packages are judgeable** | A Trial the judge silently rejects in production |
| **A public-only package carries no hidden material** | Hidden answers leaking to a volunteer worker |
| **Pinned commits, paths, and line ranges resolve** | An attribution that points at a deleted file |
| Search index builds | A broken Learn search with no compile error to warn you |

The last two run weekly as well as on every PR, because content rots without any
commit: an upstream repository can be renamed, and a cited URL can die.

## Layout

```
content/
  learn/<path>/path.json            a learning path
  learn/<path>/<lesson>.json        lesson metadata
  learn/<path>/<lesson>.md          lesson prose
  quizzes/<path>/<quiz>.json        micro quizzes
  cheatsheets/<id>.json             contextual cheatsheets
  oss/<topic>/<id>.json             curated OSS examples
  trials/<slug>/trial.json          Trial metadata and tests
  trials/<slug>/statement.md        the problem statement
  trials/<slug>/starter.rs          what the editor opens with
  trials/<slug>/solution.rs         the reference solution
schemas/                            generated from the Rust types; do not hand-edit
tools/                              the validator, checker, and index builder
```

## The schemas are generated

`schemas/*.schema.json` is produced from the Rust types in
`tools/src/model.rs`, and CI fails if the committed files differ from what the
types generate. Hand-maintaining both would guarantee drift, and a schema that
disagrees with the validator is worse than no schema.

Editors get completion and inline errors; the Rust types stay the single
authority.

```sh
cargo run --manifest-path tools/Cargo.toml -- schemas          # regenerate
cargo run --manifest-path tools/Cargo.toml -- schemas --check  # verify
```

## Working on content

```sh
cargo build --release -p rustly-content
T=./target/release/rustly-content

$T validate         # schema, identifiers, references, attribution
$T check-examples   # compile every example, assert its declared outcome
$T check-trials     # starters fail as promised, solutions pass every test
$T index            # build dist/search-index.json
$T package <slug>   # emit the judge Trial package
```

`check-examples` and `check-trials` shell out to `rustc` directly. Examples are
dependency-free by design, so nothing is resolved and no network is needed.

## Rules that are enforced, not just requested

**Every OSS example is pinned to a full 40-character commit SHA.** Never a
branch, never a tag. A branch moves, and an excerpt that pointed at lines 170-175
last month now points at something else. CI additionally resolves the commit, the
path, and the line range against the upstream repository, so an example that
outlives its source fails the build rather than teaching a lie.

**Every wrong quiz option explains why it is wrong.** A learner who chose it had
a reason, and "incorrect" addresses none of it.

**Every runnable example declares its expected outcome** — `runs` with expected
stdout, `compiles`, or `compile_error` with a specific code such as `E0382`. That
is what lets a deliberately broken example be tested: without the declaration, CI
cannot tell "meant to fail" from "broken".

**Every Trial needs at least one public test**, and CI warns when it has no
hidden test, because a solution tuned to the visible cases would pass.

**Original prose only.** We do not paste text from copyrighted books. Lessons
cite their sources in `references`; the words are ours.

## Licensing

Content — lesson prose, quizzes, Trial statements, cheatsheets — is
[CC BY 4.0](LICENSE-CONTENT).

Tooling in `tools/` is dual-licensed [MIT](LICENSE-MIT) or
[Apache-2.0](LICENSE-APACHE).

Curated OSS excerpts remain under their upstream licenses. Every example records
the repository, commit, path, line range, license, and required attribution.
Upstream snippets are never relicensed.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and the
[org-wide contribution rules](https://github.com/rustly-tech/.github/blob/main/CONTRIBUTING.md).
