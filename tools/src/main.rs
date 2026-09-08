//! `rustly-content` — validation and build tooling for the Rustly content repository.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context as _};
use clap::{Parser, Subcommand};
use rustly_content::index;
use rustly_content::model::*;
use rustly_content::validate::{self, Severity};

#[derive(Debug, Parser)]
#[command(name = "rustly-content", version, about)]
struct Args {
    /// Repository root.
    #[arg(long, default_value = ".", global = true)]
    root: PathBuf,

    #[command(subcommand)]
    command: Command_,
}

#[derive(Debug, Subcommand)]
enum Command_ {
    /// Validate every document: schema, identifiers, references, attribution.
    Validate {
        /// Treat warnings as errors.
        #[arg(long)]
        deny_warnings: bool,
    },

    /// Compile every runnable example and assert it does what it claims.
    CheckExamples,

    /// Build and run every Trial's reference solution against its own tests,
    /// and check that each starter behaves as declared.
    CheckTrials,

    /// Regenerate `schemas/` from the Rust types.
    Schemas {
        /// Fail instead of writing if the committed schemas are out of date.
        #[arg(long)]
        check: bool,
    },

    /// Build the static search index.
    Index {
        /// Where to write it.
        #[arg(long, default_value = "dist/search-index.json")]
        out: PathBuf,
    },

    /// Emit a judge Trial package for one Trial.
    Package {
        /// Trial slug.
        slug: String,
        /// Emit public tests only, as a non-trusted worker would receive.
        #[arg(long)]
        public_only: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let root = args
        .root
        .canonicalize()
        .context("resolving the repository root")?;

    match args.command {
        Command_::Validate { deny_warnings } => {
            let mut report = validate::ValidationReport::default();
            let content = validate::load(&root.join("content"), &mut report);
            validate::check(&root.join("content"), &content, &mut report);

            for diagnostic in &report.diagnostics {
                println!("{diagnostic}");
            }
            println!(
                "\n{} document(s): {} error(s), {} warning(s)",
                report.documents,
                report.error_count(),
                report.diagnostics.len() - report.error_count()
            );

            if report.has_errors() {
                bail!("validation failed");
            }
            if deny_warnings && !report.diagnostics.is_empty() {
                bail!("warnings are denied");
            }
            println!(
                "ok: {} path(s), {} lesson(s), {} quiz(zes), {} cheatsheet(s), {} OSS example(s), {} Trial(s)",
                content.paths.len(),
                content.lessons.len(),
                content.quizzes.len(),
                content.cheatsheets.len(),
                content.oss.len(),
                content.trials.len()
            );
        }

        Command_::CheckExamples => {
            let mut report = validate::ValidationReport::default();
            let content = validate::load(&root.join("content"), &mut report);
            if report.has_errors() {
                for d in &report.diagnostics {
                    println!("{d}");
                }
                bail!("fix validation errors before checking examples");
            }

            let workspace = tempfile::tempdir()?;
            let mut checked = 0;
            let mut failures = 0;

            for (lesson_id, (_, lesson)) in &content.lessons {
                for example in &lesson.examples {
                    checked += 1;
                    let label = format!("{lesson_id}#{}", example.id);
                    match check_example(workspace.path(), &example.code, &example.expect) {
                        Ok(()) => println!("ok    {label}"),
                        Err(why) => {
                            failures += 1;
                            println!("FAIL  {label}\n      {why}");
                        }
                    }
                }
            }

            println!("\n{checked} example(s) checked, {failures} failure(s)");
            if failures > 0 {
                bail!("example checks failed");
            }
        }

        Command_::CheckTrials => {
            let mut report = validate::ValidationReport::default();
            let content = validate::load(&root.join("content"), &mut report);
            if report.has_errors() {
                for d in &report.diagnostics {
                    println!("{d}");
                }
                bail!("fix validation errors before checking Trials");
            }

            let workspace = tempfile::tempdir()?;
            let mut failures = 0;

            for (slug, (path, trial)) in &content.trials {
                let directory = path.parent().unwrap_or(&root);

                // The starter must behave exactly as the Trial claims. A starter
                // that already compiles when it is meant to fail with E0382
                // removes the entire point of the exercise.
                let starter = std::fs::read_to_string(directory.join(&trial.starter))
                    .with_context(|| format!("reading the starter for {slug}"))?;
                match check_example(workspace.path(), &starter, &trial.starter_expect) {
                    Ok(()) => println!("ok    {slug} starter behaves as declared"),
                    Err(why) => {
                        failures += 1;
                        println!("FAIL  {slug} starter\n      {why}");
                    }
                }

                // The reference solution must pass every test, including hidden
                // ones. A Trial whose own solution fails is unshippable.
                let solution =
                    std::fs::read_to_string(directory.join(&trial.reference_solution))
                        .with_context(|| format!("reading the reference solution for {slug}"))?;
                match run_against_tests(workspace.path(), &solution, &trial.tests) {
                    Ok(()) => println!(
                        "ok    {slug} reference solution passes {} test(s)",
                        trial.tests.len()
                    ),
                    Err(why) => {
                        failures += 1;
                        println!("FAIL  {slug} reference solution\n      {why}");
                    }
                }
            }

            println!(
                "\n{} Trial(s) checked, {failures} failure(s)",
                content.trials.len()
            );
            if failures > 0 {
                bail!("Trial checks failed");
            }
        }

        Command_::Schemas { check } => {
            let directory = root.join("schemas");
            std::fs::create_dir_all(&directory)?;
            let generated = generate_schemas();

            let mut stale = Vec::new();
            for (name, schema) in &generated {
                let file = directory.join(format!("{name}.schema.json"));
                let existing = std::fs::read_to_string(&file).unwrap_or_default();
                if existing.trim() == schema.trim() {
                    continue;
                }
                if check {
                    stale.push(name.clone());
                } else {
                    std::fs::write(&file, schema)?;
                    println!("wrote schemas/{name}.schema.json");
                }
            }

            if check {
                if stale.is_empty() {
                    println!("ok: {} schema(s) match the Rust types", generated.len());
                } else {
                    for name in &stale {
                        println!("stale: schemas/{name}.schema.json");
                    }
                    bail!(
                        "schemas are out of date; run `rustly-content schemas` and commit the result"
                    );
                }
            }
        }

        Command_::Index { out } => {
            let mut report = validate::ValidationReport::default();
            let content = validate::load(&root.join("content"), &mut report);
            if report.has_errors() {
                for d in &report.diagnostics {
                    if d.severity == Severity::Error {
                        println!("{d}");
                    }
                }
                bail!("refusing to index content that does not validate");
            }

            let built = index::build(&content);
            let destination = if out.is_absolute() {
                out
            } else {
                root.join(out)
            };
            let bytes = index::write(&built, &destination)?;
            println!(
                "wrote {} ({} entries, {bytes} bytes)",
                destination.display(),
                built.entries.len()
            );
        }

        Command_::Package { slug, public_only } => {
            let mut report = validate::ValidationReport::default();
            let content = validate::load(&root.join("content"), &mut report);
            let Some((path, trial)) = content.trials.get(&slug) else {
                bail!("no Trial with slug `{slug}`");
            };
            let package = build_package(path.parent().unwrap_or(&root), trial, public_only)?;
            println!("{}", serde_json::to_string_pretty(&package)?);
        }
    }
    Ok(())
}

/// Compile `code` and assert it matches `expect`.
///
/// Uses `rustc` directly rather than Cargo: examples are dependency-free by
/// design, so there is nothing to resolve and no network access is needed.
fn check_example(workspace: &Path, code: &str, expect: &ExpectedOutcome) -> Result<(), String> {
    let source = workspace.join("example.rs");
    let binary = workspace.join("example.bin");
    std::fs::write(&source, code).map_err(|e| format!("cannot write the example: {e}"))?;

    let output = Command::new("rustc")
        .args(["--edition", "2021", "-A", "unused", "-o"])
        .arg(&binary)
        .arg(&source)
        .output()
        .map_err(|e| format!("cannot run rustc: {e}"))?;
    let diagnostics = String::from_utf8_lossy(&output.stderr).into_owned();

    match expect {
        ExpectedOutcome::CompileError { code: expected } => {
            if output.status.success() {
                return Err(format!(
                    "expected this example to fail with {expected}, but it compiled"
                ));
            }
            if !diagnostics.contains(expected) {
                return Err(format!(
                    "expected {expected}, got:\n{}",
                    indent(diagnostics.trim())
                ));
            }
            Ok(())
        }
        ExpectedOutcome::Compiles => {
            if output.status.success() {
                Ok(())
            } else {
                Err(format!(
                    "expected this to compile:\n{}",
                    indent(diagnostics.trim())
                ))
            }
        }
        ExpectedOutcome::Runs { stdout: expected } => {
            if !output.status.success() {
                return Err(format!(
                    "expected this to compile:\n{}",
                    indent(diagnostics.trim())
                ));
            }
            let run = Command::new(&binary)
                .output()
                .map_err(|e| format!("cannot run the example: {e}"))?;
            let actual = String::from_utf8_lossy(&run.stdout);
            if actual.trim_end() != expected.trim_end() {
                return Err(format!(
                    "expected stdout {:?}, got {:?}",
                    expected.trim_end(),
                    actual.trim_end()
                ));
            }
            Ok(())
        }
    }
}

/// Compile `code` and run it against every test case.
fn run_against_tests(workspace: &Path, code: &str, tests: &[TestCase]) -> Result<(), String> {
    use std::io::Write as _;

    let source = workspace.join("solution.rs");
    let binary = workspace.join("solution.bin");
    std::fs::write(&source, code).map_err(|e| format!("cannot write the solution: {e}"))?;

    let build = Command::new("rustc")
        .args(["--edition", "2021", "-O", "-o"])
        .arg(&binary)
        .arg(&source)
        .output()
        .map_err(|e| format!("cannot run rustc: {e}"))?;
    if !build.status.success() {
        return Err(format!(
            "the reference solution does not compile:\n{}",
            indent(String::from_utf8_lossy(&build.stderr).trim())
        ));
    }

    for test in tests {
        let mut child = Command::new(&binary)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("cannot run the solution: {e}"))?;
        child
            .stdin
            .as_mut()
            .expect("stdin was piped")
            .write_all(test.stdin.as_bytes())
            .map_err(|e| format!("cannot write stdin for `{}`: {e}", test.id))?;

        let output = child
            .wait_with_output()
            .map_err(|e| format!("cannot collect output for `{}`: {e}", test.id))?;
        if !output.status.success() {
            return Err(format!("test `{}`: the solution exited non-zero", test.id));
        }

        // Same comparison the judge's default checker uses: trailing whitespace
        // on each line is ignored, so a checker mismatch here cannot make CI
        // disagree with production judging.
        let actual = String::from_utf8_lossy(&output.stdout);
        if normalise(&actual) != normalise(&test.expected_stdout) {
            return Err(format!(
                "test `{}`: expected {:?}, got {:?}",
                test.id,
                test.expected_stdout.trim_end(),
                actual.trim_end()
            ));
        }
    }
    Ok(())
}

fn normalise(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = text.lines().map(|l| l.trim_end().to_owned()).collect();
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines
}

fn indent(text: &str) -> String {
    text.lines()
        .map(|l| format!("      {l}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The judge Trial package, built from a Trial directory.
#[derive(Debug, serde::Serialize)]
struct JudgePackage {
    format_version: u32,
    slug: String,
    version: u32,
    mode: &'static str,
    environment: JudgeEnvironment,
    limits: Limits,
    checker: serde_json::Value,
    scoring: &'static str,
    fail_fast: bool,
    groups: Vec<serde_json::Value>,
    tests: Vec<JudgeTest>,
}

#[derive(Debug, serde::Serialize)]
struct JudgeEnvironment {
    id: &'static str,
    edition: &'static str,
    toolchain: &'static str,
    target: &'static str,
}

#[derive(Debug, serde::Serialize)]
struct JudgeTest {
    id: String,
    visibility: &'static str,
    stdin: String,
    expected_stdout: String,
}

fn build_package(
    _directory: &Path,
    trial: &Trial,
    public_only: bool,
) -> anyhow::Result<JudgePackage> {
    let tests = trial
        .tests
        .iter()
        .filter(|t| !public_only || t.visibility == Visibility::Public)
        .map(|t| JudgeTest {
            id: t.id.clone(),
            visibility: match t.visibility {
                Visibility::Public => "public",
                Visibility::Hidden => "hidden",
            },
            stdin: t.stdin.clone(),
            expected_stdout: t.expected_stdout.clone(),
        })
        .collect();

    Ok(JudgePackage {
        format_version: 1,
        slug: trial.slug.clone(),
        version: trial.version,
        mode: "batch",
        environment: JudgeEnvironment {
            id: "rust-1.88-wasm32-wasip1",
            edition: "2021",
            toolchain: "1.88",
            target: "wasm32-wasip1",
        },
        limits: trial.limits.unwrap_or_default(),
        checker: serde_json::json!({ "kind": "trimmed_lines" }),
        scoring: "all_or_nothing",
        fail_fast: true,
        groups: vec![],
        tests,
    })
}

/// Generate a JSON Schema for each top-level content type.
fn generate_schemas() -> Vec<(String, String)> {
    fn one<T: schemars::JsonSchema>(name: &str) -> (String, String) {
        let schema = schemars::schema_for!(T);
        let mut text = serde_json::to_string_pretty(&schema).expect("a schema is serialisable");
        text.push('\n');
        (name.to_owned(), text)
    }

    vec![
        one::<LearningPath>("learning-path"),
        one::<Lesson>("lesson"),
        one::<Quiz>("quiz"),
        one::<Cheatsheet>("cheatsheet"),
        one::<OssExample>("oss-example"),
        one::<Trial>("trial"),
    ]
}
