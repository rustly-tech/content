# The content model

Reference for the document types. The authoritative definition is
`tools/src/model.rs`; `schemas/` is generated from it.

## Format versioning

Every document carries `format_version`, currently `1`. It is independent of any
repository version. Changing the meaning of a field without bumping it is a
breaking-change incident.

## Learning path — `content/learn/<id>/path.json`

An ordered list of lesson ids, plus prerequisites. A path with no lessons is an
error; a lesson in no path is a warning, because it is unreachable.

## Lesson — `content/learn/<path>/<id>.json` plus a `.md` body

Metadata and structure live in JSON; the prose lives in Markdown beside it. That
split keeps prose reviewable as prose in a diff, rather than as escaped strings.

A lesson may carry:

- `examples` — runnable programs, each with an `expect`
- `visualisation` — a mental-model animation the web application renders
- references to quizzes, cheatsheets, OSS examples, and Trials
- `references` — citations

### `expect`

```json
{ "kind": "runs", "stdout": "5 5" }
{ "kind": "compiles" }
{ "kind": "compile_error", "code": "E0382" }
```

Declaring the expectation is what makes a broken-on-purpose example testable.
Without it CI cannot tell "meant to fail" from "broken", and predict-the-error
teaching degrades into noise.

### `visualisation`

A `kind` (`ownership`, `borrows`, `lifetimes`), the code, and ordered steps. Each
step names a 1-based line, a note about what is true after it runs, and the state
of each binding: `owns`, `moved`, `borrowed_shared`, `borrowed_unique`,
`dropped`, `uninitialised`.

The visualisation is authored, not inferred. Inferring it would need a borrow
checker, and an approximate one teaching an exact rule is worse than nothing.

## Quiz — `content/quizzes/<path>/<id>.json`

Exactly one correct option, and every wrong option carries `why_wrong`. The
`explanation` is shown regardless of the answer.

## Cheatsheet — `content/cheatsheets/<id>.json`

A list of `want` / `code` / optional `note`. Organised by what a reader is trying
to do, not by language feature — a cheatsheet is consulted mid-task.

## OSS example — `content/oss/<topic>/<id>.json`

Required: `repository`, `commit` (40 lowercase hex), `path`, `line_start`,
`line_end`, `license`, `attribution`, `topics`. Optional `simplified` rewrite.

CI resolves the commit, path, and line range against the upstream repository, so
an example that outlives its source fails the build.

## Trial — `content/trials/<slug>/trial.json`

Metadata, `starter_expect`, and test cases. Each test has an `id`, a
`visibility` (`public` or `hidden`), `stdin`, and `expected_stdout`.

Hidden tests are stored here and are removed from any package sent to a
non-trusted judge worker. They are deleted rather than flagged: a flag protects
nothing once the bytes are on someone else's machine.

`rustly-content package <slug>` emits the judge's Trial package format;
`--public-only` emits the form a volunteer worker receives.

## Search index

`rustly-content index` writes `dist/search-index.json`: kind, id, title,
truncated summary, topics, and a link, for every published document. Draft
content is excluded — letting someone search for a page they cannot open is worse
than not indexing it.

The index is deliberately small and the browser searches it locally, which is how
Learn search stays a static-first read with no API request.
