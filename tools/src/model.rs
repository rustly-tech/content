//! The content model.
//!
//! Every content type is `Deserialize` (for reading the repository),
//! `Serialize` (for building the search index and Trial packages), and
//! `JsonSchema` (so `schemas/` can be generated rather than maintained).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Format version for every content document in this repository.
pub const CONTENT_FORMAT_VERSION: u32 = 1;

/// Difficulty tier, matching the control plane's vocabulary.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Difficulty {
    /// First contact with a concept.
    Intro,
    /// Routine application of one concept.
    Easy,
    /// Combines concepts or needs a non-obvious step.
    Medium,
    /// Requires real design judgement.
    Hard,
    /// Deep or adversarial.
    Expert,
}

/// A topic tag.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Topic {
    /// Ownership, moves, `Copy`.
    Ownership,
    /// Shared and unique borrows.
    Borrowing,
    /// Lifetimes and variance.
    Lifetimes,
    /// Traits and generics.
    Traits,
    /// Enums, pattern matching, `Option`/`Result`.
    Enums,
    /// Collections and iterators.
    Collections,
    /// Error handling.
    Errors,
    /// Concurrency and `Send`/`Sync`.
    Concurrency,
    /// Unsafe and FFI.
    Unsafe,
    /// Modules, crates, Cargo.
    Tooling,
}

/// Editorial state.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Lifecycle {
    /// Author is still working; not published.
    Draft,
    /// Published with a beta marker; does not count for ranking.
    Beta,
    /// Reviewed and accepted.
    Verified,
    /// Curated by maintainers as canonical for a concept.
    Official,
}

/// A learning path: an ordered sequence of lessons.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LearningPath {
    /// Format version.
    pub format_version: u32,
    /// Stable identifier, e.g. `ownership`.
    pub id: String,
    /// Human title.
    pub title: String,
    /// One-sentence summary shown on the Learn index.
    pub summary: String,
    /// Topics this path covers.
    pub topics: Vec<Topic>,
    /// Editorial state.
    pub lifecycle: Lifecycle,
    /// Lesson ids, in teaching order.
    pub lessons: Vec<String>,
    /// Paths a learner should complete first.
    #[serde(default)]
    pub prerequisites: Vec<String>,
}

/// What a runnable example is expected to do when compiled.
///
/// Declaring the expectation is what makes a broken-on-purpose example testable.
/// Without it, CI cannot tell "this example is meant to fail" from "this example
/// is broken", and the teaching value of a deliberate error is exactly that it
/// is deliberate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ExpectedOutcome {
    /// Compiles and runs, producing this stdout.
    Runs {
        /// Expected stdout, compared after trimming trailing whitespace.
        stdout: String,
    },
    /// Compiles cleanly. Output is not asserted.
    Compiles,
    /// Fails to compile with this diagnostic code, e.g. `E0382`.
    ///
    /// This is the backbone of predict-the-error teaching.
    CompileError {
        /// The `rustc` error code the example must produce.
        code: String,
    },
}

/// A runnable code example inside a lesson.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Example {
    /// Identifier, unique within the lesson.
    pub id: String,
    /// One-line description of what the reader should notice.
    pub caption: String,
    /// The code, as a complete compilable program.
    pub code: String,
    /// What it is expected to do.
    pub expect: ExpectedOutcome,
}

/// A micro quiz question.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Quiz {
    /// Format version.
    pub format_version: u32,
    /// Stable identifier.
    pub id: String,
    /// Lesson this quiz belongs to.
    pub lesson: String,
    /// The question.
    pub prompt: String,
    /// Optional code the question is about.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// Answer options.
    pub options: Vec<QuizOption>,
    /// Shown after answering, whether right or wrong.
    pub explanation: String,
}

/// One quiz answer option.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct QuizOption {
    /// The option text.
    pub text: String,
    /// Whether this option is correct.
    pub correct: bool,
    /// Why this option is wrong, shown when it is chosen.
    ///
    /// Required for wrong options: "incorrect" teaches nothing, and a learner
    /// who picked it had a reason worth addressing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why_wrong: Option<String>,
}

/// A lesson.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Lesson {
    /// Format version.
    pub format_version: u32,
    /// Stable identifier, e.g. `ownership/move-semantics`.
    pub id: String,
    /// Human title.
    pub title: String,
    /// One-sentence summary.
    pub summary: String,
    /// Topics.
    pub topics: Vec<Topic>,
    /// Editorial state.
    pub lifecycle: Lifecycle,
    /// Path to the Markdown body, relative to the lesson file.
    pub body: String,
    /// Mental-model visualisation to render, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visualisation: Option<Visualisation>,
    /// Runnable examples.
    #[serde(default)]
    pub examples: Vec<Example>,
    /// Quiz ids attached to this lesson.
    #[serde(default)]
    pub quizzes: Vec<String>,
    /// Cheatsheet ids to surface alongside this lesson.
    #[serde(default)]
    pub cheatsheets: Vec<String>,
    /// OSS example ids to surface alongside this lesson.
    #[serde(default)]
    pub oss_examples: Vec<String>,
    /// Trial slugs this lesson prepares the reader for.
    #[serde(default)]
    pub trials: Vec<String>,
    /// References. Original prose only; these are sources, not sources of text.
    #[serde(default)]
    pub references: Vec<Reference>,
}

/// A mental-model visualisation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Visualisation {
    /// Which visualisation the web application should render.
    pub kind: VisualisationKind,
    /// The program the visualisation steps through.
    pub code: String,
    /// Ordered steps, each describing the state after one line executes.
    pub steps: Vec<VisualisationStep>,
}

/// The kinds of visualisation the web application knows how to render.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VisualisationKind {
    /// Stack and heap cells, showing who owns what.
    Ownership,
    /// Active borrows over a span of lines.
    Borrows,
    /// Lifetime extents.
    Lifetimes,
}

/// One step of a visualisation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct VisualisationStep {
    /// 1-based line in [`Visualisation::code`] this step describes.
    pub line: u32,
    /// What is true after that line runs.
    pub note: String,
    /// Named bindings and their state after that line.
    #[serde(default)]
    pub bindings: Vec<Binding>,
}

/// A binding's state at one step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Binding {
    /// Variable name.
    pub name: String,
    /// What it holds, or why it cannot be used.
    pub state: BindingState,
}

/// What a binding is doing at a point in the program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BindingState {
    /// Owns its value.
    Owns,
    /// Was moved out of and can no longer be used.
    Moved,
    /// Holds a shared reference.
    BorrowedShared,
    /// Holds a unique reference.
    BorrowedUnique,
    /// Out of scope; its value has been dropped.
    Dropped,
    /// Declared but not yet initialised.
    Uninitialised,
}

/// A citation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Reference {
    /// What is being cited.
    pub title: String,
    /// Where to find it.
    pub url: String,
}

/// A cheatsheet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Cheatsheet {
    /// Format version.
    pub format_version: u32,
    /// Stable identifier.
    pub id: String,
    /// Human title.
    pub title: String,
    /// One-sentence summary.
    pub summary: String,
    /// Topics.
    pub topics: Vec<Topic>,
    /// Editorial state.
    pub lifecycle: Lifecycle,
    /// Entries.
    pub entries: Vec<CheatsheetEntry>,
}

/// One cheatsheet entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CheatsheetEntry {
    /// What the reader is trying to do.
    pub want: String,
    /// The code that does it.
    pub code: String,
    /// A short note, when the code alone is not enough.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// A curated example from an open-source Rust project.
///
/// Every field is required because attribution is not optional and a moving
/// branch is not a citation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct OssExample {
    /// Format version.
    pub format_version: u32,
    /// Stable identifier.
    pub id: String,
    /// What the reader should notice in this code.
    pub caption: String,
    /// Upstream repository, `owner/name`.
    pub repository: String,
    /// The exact commit this was taken from. Forty hex characters, never a branch.
    pub commit: String,
    /// Path within the repository at that commit.
    pub path: String,
    /// First line of the excerpt, 1-based and inclusive.
    pub line_start: u32,
    /// Last line of the excerpt, inclusive.
    pub line_end: u32,
    /// Upstream SPDX license expression.
    pub license: String,
    /// Attribution line to display.
    pub attribution: String,
    /// Concept tags.
    pub topics: Vec<Topic>,
    /// A simplified rewrite of the same idea, for readers not ready for the original.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub simplified: Option<String>,
}

impl OssExample {
    /// A permalink to the exact lines at the pinned commit.
    pub fn permalink(&self) -> String {
        format!(
            "https://github.com/{}/blob/{}/{}#L{}-L{}",
            self.repository, self.commit, self.path, self.line_start, self.line_end
        )
    }
}

/// A Trial: the judged practice unit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Trial {
    /// Format version.
    pub format_version: u32,
    /// URL slug and identifier.
    pub slug: String,
    /// Human title.
    pub title: String,
    /// One-sentence summary.
    pub summary: String,
    /// Difficulty tier.
    pub difficulty: Difficulty,
    /// Topics.
    pub topics: Vec<Topic>,
    /// Editorial state.
    pub lifecycle: Lifecycle,
    /// Content version. Bump whenever tests or the statement change meaningfully.
    pub version: u32,
    /// Path to the Markdown statement, relative to the Trial file.
    pub statement: String,
    /// Path to the starter code the editor opens with.
    pub starter: String,
    /// What the starter is expected to do before the learner changes anything.
    pub starter_expect: ExpectedOutcome,
    /// Public test cases. Trusted evaluation lives outside this repository.
    pub tests: Vec<TestCase>,
    /// Resource limits, if the defaults are not right for this Trial.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limits: Option<Limits>,
}

/// One Trial test case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TestCase {
    /// Identifier, unique within the Trial.
    pub id: String,
    /// Bytes written to the program's stdin.
    pub stdin: String,
    /// Expected stdout.
    pub expected_stdout: String,
}

/// Resource limits, mirroring the judge's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Limits {
    /// Wall-clock limit per execution, milliseconds.
    pub wall_ms: u64,
    /// Memory limit, bytes.
    pub memory_bytes: u64,
    /// Combined stdout+stderr limit, bytes.
    pub output_bytes: u64,
    /// Abstract work bound.
    pub fuel: u64,
    /// Maximum guest table elements.
    pub table_elements: u64,
    /// Maximum concurrent guest instances.
    pub instances: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            wall_ms: 2_000,
            memory_bytes: 64 * 1024 * 1024,
            output_bytes: 256 * 1024,
            fuel: 500_000_000,
            table_elements: 10_000,
            instances: 1,
        }
    }
}
