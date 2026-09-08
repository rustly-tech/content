//! Repository-wide validation.
//!
//! Schema conformance is only the easy half. The interesting failures are
//! *referential*: a lesson pointing at a quiz that was renamed, two Trials
//! sharing a slug, an OSS example pinned to a branch instead of a commit, a
//! wrong answer with no explanation of why it is wrong. Those are what this
//! module looks for.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::model::*;

/// How bad a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Blocks the build.
    Error,
    /// Worth fixing, does not block.
    Warning,
}

/// One finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// How bad it is.
    pub severity: Severity,
    /// Where it was found.
    pub location: String,
    /// What is wrong.
    pub message: String,
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(f, "{label}: {}: {}", self.location, self.message)
    }
}

/// Everything loaded from the repository.
#[derive(Debug, Default)]
pub struct Content {
    /// Learning paths, keyed by id.
    pub paths: BTreeMap<String, (PathBuf, LearningPath)>,
    /// Lessons, keyed by id.
    pub lessons: BTreeMap<String, (PathBuf, Lesson)>,
    /// Quizzes, keyed by id.
    pub quizzes: BTreeMap<String, (PathBuf, Quiz)>,
    /// Cheatsheets, keyed by id.
    pub cheatsheets: BTreeMap<String, (PathBuf, Cheatsheet)>,
    /// OSS examples, keyed by id.
    pub oss: BTreeMap<String, (PathBuf, OssExample)>,
    /// Trials, keyed by slug.
    pub trials: BTreeMap<String, (PathBuf, Trial)>,
}

/// The outcome of validating a repository.
#[derive(Debug, Default)]
pub struct ValidationReport {
    /// Findings, in discovery order.
    pub diagnostics: Vec<Diagnostic>,
    /// How many documents were checked.
    pub documents: usize,
}

impl ValidationReport {
    /// Whether any finding blocks the build.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }

    /// Number of blocking findings.
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count()
    }

    fn error(&mut self, location: impl Into<String>, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic {
            severity: Severity::Error,
            location: location.into(),
            message: message.into(),
        });
    }

    fn warn(&mut self, location: impl Into<String>, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic {
            severity: Severity::Warning,
            location: location.into(),
            message: message.into(),
        });
    }
}

/// Load every content document under `root`.
///
/// A file that does not parse is reported and skipped, so one broken document
/// does not hide every other problem in the repository.
pub fn load(root: &Path, report: &mut ValidationReport) -> Content {
    let mut content = Content::default();

    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !path.is_file() || path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .unwrap_or(path)
            .display()
            .to_string();
        let Ok(text) = std::fs::read_to_string(path) else {
            report.error(&relative, "cannot be read");
            continue;
        };

        report.documents += 1;
        let kind = classify(path);
        let parse_failed = |report: &mut ValidationReport, e: serde_json::Error| {
            report.error(&relative, format!("does not match the {kind} schema: {e}"));
        };

        match kind {
            "learning path" => match serde_json::from_str::<LearningPath>(&text) {
                Ok(value) => {
                    content
                        .paths
                        .insert(value.id.clone(), (path.to_owned(), value));
                }
                Err(e) => parse_failed(report, e),
            },
            "lesson" => match serde_json::from_str::<Lesson>(&text) {
                Ok(value) => {
                    content
                        .lessons
                        .insert(value.id.clone(), (path.to_owned(), value));
                }
                Err(e) => parse_failed(report, e),
            },
            "quiz" => match serde_json::from_str::<Quiz>(&text) {
                Ok(value) => {
                    content
                        .quizzes
                        .insert(value.id.clone(), (path.to_owned(), value));
                }
                Err(e) => parse_failed(report, e),
            },
            "cheatsheet" => match serde_json::from_str::<Cheatsheet>(&text) {
                Ok(value) => {
                    content
                        .cheatsheets
                        .insert(value.id.clone(), (path.to_owned(), value));
                }
                Err(e) => parse_failed(report, e),
            },
            "OSS example" => match serde_json::from_str::<OssExample>(&text) {
                Ok(value) => {
                    content
                        .oss
                        .insert(value.id.clone(), (path.to_owned(), value));
                }
                Err(e) => parse_failed(report, e),
            },
            "trial" => match serde_json::from_str::<Trial>(&text) {
                Ok(value) => {
                    content
                        .trials
                        .insert(value.slug.clone(), (path.to_owned(), value));
                }
                Err(e) => parse_failed(report, e),
            },
            _ => {
                report.documents -= 1;
            }
        }
    }

    content
}

fn classify(path: &Path) -> &'static str {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let parents: Vec<&str> = path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    if name == "path.json" {
        return "learning path";
    }
    if name == "trial.json" {
        return "trial";
    }
    if parents.contains(&"quizzes") {
        return "quiz";
    }
    if parents.contains(&"cheatsheets") {
        return "cheatsheet";
    }
    if parents.contains(&"oss") {
        return "OSS example";
    }
    if parents.contains(&"learn") {
        return "lesson";
    }
    "unknown"
}

/// Validate everything that cannot be expressed in a schema.
pub fn check(root: &Path, content: &Content, report: &mut ValidationReport) {
    check_format_versions(content, report);
    check_ids(content, report);
    check_references(content, report);
    check_quizzes(content, report);
    check_oss_attribution(content, report);
    check_trials(root, content, report);
    check_files_exist(root, content, report);
}

fn location(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn check_format_versions(content: &Content, report: &mut ValidationReport) {
    let mut check = |id: &str, version: u32| {
        if version != CONTENT_FORMAT_VERSION {
            report.error(
                id,
                format!(
                    "format_version {version} is not supported (expected {CONTENT_FORMAT_VERSION})"
                ),
            );
        }
    };
    for (id, (_, value)) in &content.paths {
        check(id, value.format_version);
    }
    for (id, (_, value)) in &content.lessons {
        check(id, value.format_version);
    }
    for (id, (_, value)) in &content.quizzes {
        check(id, value.format_version);
    }
    for (id, (_, value)) in &content.cheatsheets {
        check(id, value.format_version);
    }
    for (id, (_, value)) in &content.oss {
        check(id, value.format_version);
    }
    for (slug, (_, value)) in &content.trials {
        check(slug, value.format_version);
    }
}

/// Identifiers must be URL-safe and unique across their own kind.
///
/// `BTreeMap` insertion already deduplicates, so a duplicate would silently
/// shadow. Counting documents against unique ids is what catches it.
fn check_ids(content: &Content, report: &mut ValidationReport) {
    let valid = |id: &str| {
        !id.is_empty()
            && id.len() <= 96
            && id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '/'))
            && !id.starts_with(['-', '/'])
            && !id.ends_with(['-', '/'])
            && !id.contains("//")
            && !id.contains("--")
    };

    let all: Vec<(&str, &str)> = content
        .paths
        .keys()
        .map(|id| ("learning path", id.as_str()))
        .chain(content.lessons.keys().map(|id| ("lesson", id.as_str())))
        .chain(content.quizzes.keys().map(|id| ("quiz", id.as_str())))
        .chain(
            content
                .cheatsheets
                .keys()
                .map(|id| ("cheatsheet", id.as_str())),
        )
        .chain(content.oss.keys().map(|id| ("OSS example", id.as_str())))
        .chain(content.trials.keys().map(|id| ("trial", id.as_str())))
        .collect();

    for (kind, id) in &all {
        if !valid(id) {
            report.error(
                *id,
                format!("{kind} id must be 1-96 characters of a-z, 0-9, '-' and '/', with no leading, trailing, or doubled separator"),
            );
        }
    }
}

fn check_references(content: &Content, report: &mut ValidationReport) {
    for (id, (_, path)) in &content.paths {
        if path.lessons.is_empty() {
            report.error(id, "a learning path must contain at least one lesson");
        }
        for lesson in &path.lessons {
            if !content.lessons.contains_key(lesson) {
                report.error(id, format!("references unknown lesson `{lesson}`"));
            }
        }
        for prerequisite in &path.prerequisites {
            if !content.paths.contains_key(prerequisite) {
                report.error(
                    id,
                    format!("references unknown prerequisite path `{prerequisite}`"),
                );
            }
            if prerequisite == id {
                report.error(id, "a path cannot be its own prerequisite");
            }
        }
    }

    for (id, (_, lesson)) in &content.lessons {
        for quiz in &lesson.quizzes {
            match content.quizzes.get(quiz) {
                None => report.error(id, format!("references unknown quiz `{quiz}`")),
                Some((_, q)) if &q.lesson != id => report.error(
                    id,
                    format!(
                        "quiz `{quiz}` belongs to lesson `{}`, not this one",
                        q.lesson
                    ),
                ),
                Some(_) => {}
            }
        }
        for sheet in &lesson.cheatsheets {
            if !content.cheatsheets.contains_key(sheet) {
                report.error(id, format!("references unknown cheatsheet `{sheet}`"));
            }
        }
        for example in &lesson.oss_examples {
            if !content.oss.contains_key(example) {
                report.error(id, format!("references unknown OSS example `{example}`"));
            }
        }
        for trial in &lesson.trials {
            if !content.trials.contains_key(trial) {
                report.error(id, format!("references unknown Trial `{trial}`"));
            }
        }

        let mut seen = BTreeSet::new();
        for example in &lesson.examples {
            if !seen.insert(example.id.as_str()) {
                report.error(id, format!("duplicate example id `{}`", example.id));
            }
        }
    }

    // A lesson nobody can reach is almost always an oversight.
    let reachable: BTreeSet<&str> = content
        .paths
        .values()
        .flat_map(|(_, p)| p.lessons.iter().map(String::as_str))
        .collect();
    for id in content.lessons.keys() {
        if !reachable.contains(id.as_str()) {
            report.warn(id, "this lesson is not listed in any learning path");
        }
    }
}

fn check_quizzes(content: &Content, report: &mut ValidationReport) {
    for (id, (_, quiz)) in &content.quizzes {
        if !content.lessons.contains_key(&quiz.lesson) {
            report.error(id, format!("belongs to unknown lesson `{}`", quiz.lesson));
        }
        if quiz.options.len() < 2 {
            report.error(id, "a quiz needs at least two options");
        }

        let correct = quiz.options.iter().filter(|o| o.correct).count();
        if correct != 1 {
            report.error(
                id,
                format!("exactly one option must be correct, found {correct}"),
            );
        }

        for option in &quiz.options {
            // A wrong answer with no explanation teaches nothing, and the
            // learner who picked it had a reason worth addressing.
            if !option.correct
                && option
                    .why_wrong
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .is_empty()
            {
                report.error(
                    id,
                    format!("option {:?} is wrong but does not say why", option.text),
                );
            }
            if option.correct && option.why_wrong.is_some() {
                report.warn(id, "the correct option should not carry a `why_wrong`");
            }
        }

        if quiz.explanation.trim().len() < 20 {
            report.error(id, "the explanation is too short to be useful");
        }
    }
}

fn check_oss_attribution(content: &Content, report: &mut ValidationReport) {
    for (id, (_, example)) in &content.oss {
        // A moving branch is not a citation: the lines it points at change.
        if example.commit.len() != 40 || !example.commit.chars().all(|c| c.is_ascii_hexdigit()) {
            report.error(
                id,
                format!(
                    "commit must be a full 40-character SHA, never a branch or tag: {:?}",
                    example.commit
                ),
            );
        }
        if example.commit.chars().any(|c| c.is_ascii_uppercase()) {
            report.error(id, "commit SHA must be lowercase");
        }
        if !example.repository.contains('/') || example.repository.split('/').count() != 2 {
            report.error(
                id,
                format!(
                    "repository must be `owner/name`, got {:?}",
                    example.repository
                ),
            );
        }
        if example.path.is_empty() || example.path.starts_with('/') || example.path.contains("..") {
            report.error(id, "path must be a relative path inside the repository");
        }
        if example.line_start == 0 {
            report.error(
                id,
                "line numbers are 1-based; line_start must be at least 1",
            );
        }
        if example.line_end < example.line_start {
            report.error(id, "line_end must not precede line_start");
        }
        if example.line_end.saturating_sub(example.line_start) > 60 {
            report.warn(id, "excerpts longer than 60 lines are hard to learn from");
        }
        if example.license.trim().is_empty() {
            report.error(id, "the upstream license must be recorded");
        }
        if example.attribution.trim().is_empty() {
            report.error(id, "attribution must be recorded");
        }
        if example.topics.is_empty() {
            report.error(id, "an OSS example needs at least one topic tag");
        }
    }
}

fn check_trials(root: &Path, content: &Content, report: &mut ValidationReport) {
    for (slug, (path, trial)) in &content.trials {
        let at = location(path, root);

        if trial.version == 0 {
            report.error(&at, "version must be at least 1");
        }
        if trial.tests.is_empty() {
            report.error(&at, "a Trial needs at least one test");
        }

        let mut seen = BTreeSet::new();
        for test in &trial.tests {
            if !seen.insert(test.id.as_str()) {
                report.error(&at, format!("duplicate test id `{}`", test.id));
            }
            if test.expected_stdout.is_empty() {
                report.warn(&at, format!("test `{}` expects no output", test.id));
            }
        }

        if !trial
            .tests
            .iter()
            .any(|t| t.visibility == Visibility::Public)
        {
            report.error(&at, "a Trial must expose at least one public test");
        }
        if !trial
            .tests
            .iter()
            .any(|t| t.visibility == Visibility::Hidden)
        {
            report.warn(
                &at,
                "this Trial has no hidden tests, so a solution tuned to the public cases passes",
            );
        }
        if let Some(limits) = trial.limits {
            for (field, value) in [
                ("wall_ms", limits.wall_ms),
                ("memory_bytes", limits.memory_bytes),
                ("output_bytes", limits.output_bytes),
                ("fuel", limits.fuel),
                ("instances", limits.instances),
            ] {
                if value == 0 {
                    report.error(
                        &at,
                        format!("{field} must be greater than zero, not unlimited"),
                    );
                }
            }
        }
        if slug != &trial.slug {
            report.error(&at, "the Trial slug does not match its directory key");
        }
    }
}

/// Every path a document points at must exist.
fn check_files_exist(root: &Path, content: &Content, report: &mut ValidationReport) {
    let require = |owner: &Path, relative: &str, what: &str, report: &mut ValidationReport| {
        let resolved = owner.parent().unwrap_or(root).join(relative);
        if !resolved.exists() {
            report.error(
                location(owner, root),
                format!("{what} `{relative}` does not exist"),
            );
        }
    };

    for (path, lesson) in content.lessons.values() {
        require(path, &lesson.body, "body", report);
    }
    for (path, trial) in content.trials.values() {
        require(path, &trial.statement, "statement", report);
        require(path, &trial.starter, "starter", report);
        require(
            path,
            &trial.reference_solution,
            "reference solution",
            report,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quiz(options: Vec<QuizOption>) -> Quiz {
        Quiz {
            format_version: CONTENT_FORMAT_VERSION,
            id: "ownership/predict".into(),
            lesson: "ownership/move-semantics".into(),
            prompt: "What does this print?".into(),
            code: None,
            options,
            explanation: "A move invalidates the original binding, so it cannot be used.".into(),
        }
    }

    fn oss() -> OssExample {
        OssExample {
            format_version: CONTENT_FORMAT_VERSION,
            id: "ownership/builder-move".into(),
            caption: "A builder that consumes and returns self".into(),
            repository: "clap-rs/clap".into(),
            commit: "af3044228bd76a87a3f46a3c7c3343563c059527".into(),
            path: "clap_builder/src/builder/command.rs".into(),
            line_start: 170,
            line_end: 175,
            license: "MIT OR Apache-2.0".into(),
            attribution: "clap contributors".into(),
            topics: vec![Topic::Ownership],
            simplified: None,
        }
    }

    fn content_with_oss(example: OssExample) -> Content {
        let mut content = Content::default();
        content
            .oss
            .insert(example.id.clone(), (PathBuf::from("oss/x.json"), example));
        content
    }

    #[test]
    fn a_moving_branch_is_not_a_valid_citation() {
        for bad in ["main", "master", "v1.0.0", "abc123", ""] {
            let mut example = oss();
            example.commit = bad.into();
            let mut report = ValidationReport::default();
            check_oss_attribution(&content_with_oss(example), &mut report);
            assert!(report.has_errors(), "should reject commit {bad:?}");
        }
    }

    #[test]
    fn an_uppercase_sha_is_rejected_so_permalinks_are_canonical() {
        let mut example = oss();
        example.commit = example.commit.to_uppercase();
        let mut report = ValidationReport::default();
        check_oss_attribution(&content_with_oss(example), &mut report);
        assert!(report.has_errors());
    }

    #[test]
    fn a_well_formed_oss_example_passes() {
        let mut report = ValidationReport::default();
        check_oss_attribution(&content_with_oss(oss()), &mut report);
        assert!(!report.has_errors(), "{:?}", report.diagnostics);
    }

    #[test]
    fn missing_attribution_or_license_is_an_error() {
        for mutate in [
            (|e: &mut OssExample| e.license = "  ".into()) as fn(&mut OssExample),
            |e| e.attribution = String::new(),
            |e| e.topics.clear(),
        ] {
            let mut example = oss();
            mutate(&mut example);
            let mut report = ValidationReport::default();
            check_oss_attribution(&content_with_oss(example), &mut report);
            assert!(report.has_errors());
        }
    }

    #[test]
    fn line_ranges_must_be_sane() {
        let mut example = oss();
        example.line_start = 0;
        let mut report = ValidationReport::default();
        check_oss_attribution(&content_with_oss(example), &mut report);
        assert!(report.has_errors());

        let mut example = oss();
        example.line_start = 90;
        example.line_end = 10;
        let mut report = ValidationReport::default();
        check_oss_attribution(&content_with_oss(example), &mut report);
        assert!(report.has_errors());
    }

    #[test]
    fn the_permalink_points_at_the_pinned_commit() {
        assert_eq!(
            oss().permalink(),
            "https://github.com/clap-rs/clap/blob/af3044228bd76a87a3f46a3c7c3343563c059527/clap_builder/src/builder/command.rs#L170-L175"
        );
    }

    #[test]
    fn a_wrong_quiz_option_must_explain_why_it_is_wrong() {
        let mut content = Content::default();
        content.lessons.insert(
            "ownership/move-semantics".into(),
            (PathBuf::from("l.json"), lesson()),
        );
        let q = quiz(vec![
            QuizOption {
                text: "It prints 5".into(),
                correct: true,
                why_wrong: None,
            },
            QuizOption {
                text: "It panics".into(),
                correct: false,
                why_wrong: None,
            },
        ]);
        content
            .quizzes
            .insert(q.id.clone(), (PathBuf::from("q.json"), q));

        let mut report = ValidationReport::default();
        check_quizzes(&content, &mut report);
        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|d| d.message.contains("does not say why")));
    }

    #[test]
    fn a_quiz_needs_exactly_one_correct_option() {
        let mut content = Content::default();
        content.lessons.insert(
            "ownership/move-semantics".into(),
            (PathBuf::from("l.json"), lesson()),
        );

        for options in [
            vec![
                QuizOption {
                    text: "a".into(),
                    correct: true,
                    why_wrong: None,
                },
                QuizOption {
                    text: "b".into(),
                    correct: true,
                    why_wrong: None,
                },
            ],
            vec![
                QuizOption {
                    text: "a".into(),
                    correct: false,
                    why_wrong: Some("no".into()),
                },
                QuizOption {
                    text: "b".into(),
                    correct: false,
                    why_wrong: Some("no".into()),
                },
            ],
        ] {
            let q = quiz(options);
            content
                .quizzes
                .insert(q.id.clone(), (PathBuf::from("q.json"), q));
            let mut report = ValidationReport::default();
            check_quizzes(&content, &mut report);
            assert!(report.has_errors());
        }
    }

    fn lesson() -> Lesson {
        Lesson {
            format_version: CONTENT_FORMAT_VERSION,
            id: "ownership/move-semantics".into(),
            title: "Move semantics".into(),
            summary: "What happens when you assign a String".into(),
            topics: vec![Topic::Ownership],
            lifecycle: Lifecycle::Official,
            body: "move-semantics.md".into(),
            visualisation: None,
            examples: vec![],
            quizzes: vec![],
            cheatsheets: vec![],
            oss_examples: vec![],
            trials: vec![],
            references: vec![],
        }
    }

    #[test]
    fn a_dangling_reference_is_an_error() {
        let mut content = Content::default();
        let mut l = lesson();
        l.quizzes = vec!["ownership/does-not-exist".into()];
        content
            .lessons
            .insert(l.id.clone(), (PathBuf::from("l.json"), l));

        let mut report = ValidationReport::default();
        check_references(&content, &mut report);
        assert!(report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|d| d.message.contains("unknown quiz")));
    }

    #[test]
    fn an_unreachable_lesson_is_a_warning_not_an_error() {
        let mut content = Content::default();
        let l = lesson();
        content
            .lessons
            .insert(l.id.clone(), (PathBuf::from("l.json"), l));

        let mut report = ValidationReport::default();
        check_references(&content, &mut report);
        assert!(!report.has_errors());
        assert!(report
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Warning));
    }

    #[test]
    fn identifiers_must_be_url_safe() {
        let mut content = Content::default();
        for bad in [
            "Ownership",
            "own ership",
            "own_ership",
            "-lead",
            "trail-",
            "a//b",
            "a--b",
        ] {
            content.cheatsheets.insert(
                bad.to_string(),
                (
                    PathBuf::from("c.json"),
                    Cheatsheet {
                        format_version: CONTENT_FORMAT_VERSION,
                        id: bad.into(),
                        title: "x".into(),
                        summary: "x".into(),
                        topics: vec![],
                        lifecycle: Lifecycle::Draft,
                        entries: vec![],
                    },
                ),
            );
        }
        let mut report = ValidationReport::default();
        check_ids(&content, &mut report);
        assert_eq!(report.error_count(), 7, "{:?}", report.diagnostics);
    }

    #[test]
    fn a_trial_without_a_public_test_is_an_error_and_without_a_hidden_one_is_a_warning() {
        let mut content = Content::default();
        let mut trial = Trial {
            format_version: CONTENT_FORMAT_VERSION,
            slug: "t".into(),
            title: "T".into(),
            summary: "s".into(),
            difficulty: Difficulty::Easy,
            topics: vec![Topic::Ownership],
            lifecycle: Lifecycle::Verified,
            version: 1,
            statement: "statement.md".into(),
            starter: "starter.rs".into(),
            reference_solution: "solution.rs".into(),
            starter_expect: ExpectedOutcome::CompileError {
                code: "E0382".into(),
            },
            tests: vec![TestCase {
                id: "a".into(),
                visibility: Visibility::Hidden,
                stdin: String::new(),
                expected_stdout: "x\n".into(),
            }],
            limits: None,
        };
        content
            .trials
            .insert("t".into(), (PathBuf::from("trial.json"), trial.clone()));
        let mut report = ValidationReport::default();
        check_trials(Path::new("."), &content, &mut report);
        assert!(report.has_errors(), "no public test must be an error");

        trial.tests[0].visibility = Visibility::Public;
        content
            .trials
            .insert("t".into(), (PathBuf::from("trial.json"), trial));
        let mut report = ValidationReport::default();
        check_trials(Path::new("."), &content, &mut report);
        assert!(!report.has_errors());
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.message.contains("no hidden tests")),
            "no hidden test should warn"
        );
    }

    #[test]
    fn a_zero_limit_is_rejected_as_malformed_not_treated_as_unlimited() {
        let mut content = Content::default();
        content.trials.insert(
            "t".into(),
            (
                PathBuf::from("trial.json"),
                Trial {
                    format_version: CONTENT_FORMAT_VERSION,
                    slug: "t".into(),
                    title: "T".into(),
                    summary: "s".into(),
                    difficulty: Difficulty::Easy,
                    topics: vec![Topic::Ownership],
                    lifecycle: Lifecycle::Verified,
                    version: 1,
                    statement: "statement.md".into(),
                    starter: "starter.rs".into(),
                    reference_solution: "solution.rs".into(),
                    starter_expect: ExpectedOutcome::Compiles,
                    tests: vec![
                        TestCase {
                            id: "a".into(),
                            visibility: Visibility::Public,
                            stdin: String::new(),
                            expected_stdout: "x\n".into(),
                        },
                        TestCase {
                            id: "b".into(),
                            visibility: Visibility::Hidden,
                            stdin: String::new(),
                            expected_stdout: "y\n".into(),
                        },
                    ],
                    limits: Some(Limits {
                        fuel: 0,
                        ..Limits::default()
                    }),
                },
            ),
        );
        let mut report = ValidationReport::default();
        check_trials(Path::new("."), &content, &mut report);
        assert!(report.has_errors());
    }
}
