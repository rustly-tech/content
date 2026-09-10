# Rustly content

Lessons and exercises for Rustly.

This repository contains the material learners use on [rustly.tech](https://rustly.tech):
interactive lessons, quizzes, Trials, cheatsheets, and selected examples from
open-source Rust projects.

## Contributing content

Start with [CONTRIBUTING.md](CONTRIBUTING.md). The
[content model](docs/CONTENT_MODEL.md) documents identifiers, versioning, source
citations, and how the different content types fit together.

Content lives under these directories:

- `lessons/` and `quizzes/` teach individual concepts.
- `trials/` contains judged exercises and their test cases.
- `cheatsheets/` contains compact references.
- `oss-examples/` contains excerpts pinned to upstream commits.
- `learning-paths/` orders material for the website.

## Validate

```sh
cargo run --manifest-path tools/Cargo.toml -- validate --root . --deny-warnings
cargo test --manifest-path tools/Cargo.toml
```

The validator checks schemas, links, examples, expected compiler diagnostics,
public Trial cases, and the generated search index.

## License

Validation code is MIT or Apache-2.0. Original learning material uses the
repository's content license. Third-party examples retain their upstream
licenses and attribution.
