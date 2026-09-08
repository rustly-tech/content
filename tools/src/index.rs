//! The static search index.
//!
//! Invariant A says an ordinary Learn, cheatsheet, or Trial read must not
//! require a database or an API request. Search is part of that: the browser
//! downloads this index once and searches locally, so typing in the search box
//! produces no network traffic at all.
//!
//! The index is therefore optimised for **size**, not for features. It carries
//! what a result card needs to render and a reference back to the document. It
//! is not a copy of the content.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::model::{Lifecycle, Topic};
use crate::validate::Content;

/// Format version of the search index.
pub const INDEX_FORMAT_VERSION: u32 = 1;

/// Maximum characters of summary kept per entry.
const MAX_SUMMARY: usize = 160;

/// What kind of thing an entry points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    /// A learning path.
    Path,
    /// A lesson.
    Lesson,
    /// A cheatsheet.
    Cheatsheet,
    /// A Trial.
    Trial,
    /// A curated OSS example.
    Oss,
}

/// One searchable entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    /// What it is.
    pub kind: EntryKind,
    /// Stable identifier or slug.
    pub id: String,
    /// Title shown on the result.
    pub title: String,
    /// Truncated summary.
    pub summary: String,
    /// Topic tags, for filtering.
    pub topics: Vec<Topic>,
    /// Relative site link.
    pub href: String,
}

/// The complete index.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchIndex {
    /// Format version.
    pub format_version: u32,
    /// Entries, sorted by kind then id so the output is byte-stable.
    pub entries: Vec<Entry>,
}

fn truncate(text: &str) -> String {
    if text.chars().count() <= MAX_SUMMARY {
        return text.to_owned();
    }
    let mut out: String = text.chars().take(MAX_SUMMARY - 1).collect();
    out.push('…');
    out
}

/// Whether a document is published and therefore searchable.
///
/// Draft content is excluded: shipping an index that lets a learner search for
/// something they cannot open is worse than not indexing it.
fn published(lifecycle: Lifecycle) -> bool {
    lifecycle != Lifecycle::Draft
}

/// Build the index from loaded content.
pub fn build(content: &Content) -> SearchIndex {
    let mut entries = Vec::new();

    for (id, (_, path)) in &content.paths {
        if !published(path.lifecycle) {
            continue;
        }
        entries.push(Entry {
            kind: EntryKind::Path,
            id: id.clone(),
            title: path.title.clone(),
            summary: truncate(&path.summary),
            topics: path.topics.clone(),
            href: format!("/learn/{id}"),
        });
    }

    for (id, (_, lesson)) in &content.lessons {
        if !published(lesson.lifecycle) {
            continue;
        }
        entries.push(Entry {
            kind: EntryKind::Lesson,
            id: id.clone(),
            title: lesson.title.clone(),
            summary: truncate(&lesson.summary),
            topics: lesson.topics.clone(),
            href: format!("/learn/{id}"),
        });
    }

    for (id, (_, sheet)) in &content.cheatsheets {
        if !published(sheet.lifecycle) {
            continue;
        }
        entries.push(Entry {
            kind: EntryKind::Cheatsheet,
            id: id.clone(),
            title: sheet.title.clone(),
            summary: truncate(&sheet.summary),
            topics: sheet.topics.clone(),
            href: format!("/cheatsheets/{id}"),
        });
    }

    for (slug, (_, trial)) in &content.trials {
        if !published(trial.lifecycle) {
            continue;
        }
        entries.push(Entry {
            kind: EntryKind::Trial,
            id: slug.clone(),
            title: trial.title.clone(),
            summary: truncate(&trial.summary),
            topics: trial.topics.clone(),
            href: format!("/trials/{slug}"),
        });
    }

    for (id, (_, example)) in &content.oss {
        entries.push(Entry {
            kind: EntryKind::Oss,
            id: id.clone(),
            title: format!("{}: {}", example.repository, example.caption),
            summary: truncate(&example.caption),
            topics: example.topics.clone(),
            href: example.permalink(),
        });
    }

    // Deterministic order, so rebuilding without content changes produces a
    // byte-identical file and the CDN keeps its cache.
    entries.sort_by(|a, b| (a.kind as u8, &a.id).cmp(&(b.kind as u8, &b.id)));

    SearchIndex {
        format_version: INDEX_FORMAT_VERSION,
        entries,
    }
}

/// Write the index as compact JSON.
pub fn write(index: &SearchIndex, destination: &Path) -> anyhow::Result<usize> {
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_vec(index)?;
    std::fs::write(destination, &json)?;
    Ok(json.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use std::path::PathBuf;

    fn content() -> Content {
        let mut content = Content::default();
        for (id, lifecycle) in [
            ("ownership/published", Lifecycle::Official),
            ("ownership/draft", Lifecycle::Draft),
        ] {
            content.lessons.insert(
                id.into(),
                (
                    PathBuf::from("l.json"),
                    Lesson {
                        format_version: CONTENT_FORMAT_VERSION,
                        id: id.into(),
                        title: "Title".into(),
                        summary: "x".repeat(400),
                        topics: vec![Topic::Ownership],
                        lifecycle,
                        body: "b.md".into(),
                        visualisation: None,
                        examples: vec![],
                        quizzes: vec![],
                        cheatsheets: vec![],
                        oss_examples: vec![],
                        trials: vec![],
                        references: vec![],
                    },
                ),
            );
        }
        content
    }

    #[test]
    fn draft_content_is_not_searchable() {
        let index = build(&content());
        let ids: Vec<&str> = index.entries.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["ownership/published"]);
    }

    #[test]
    fn summaries_are_truncated_so_the_index_stays_small() {
        let index = build(&content());
        let summary = &index.entries[0].summary;
        assert!(summary.chars().count() <= MAX_SUMMARY);
        assert!(summary.ends_with('…'));
    }

    #[test]
    fn the_index_is_byte_stable_across_rebuilds() {
        let a = serde_json::to_vec(&build(&content())).unwrap();
        let b = serde_json::to_vec(&build(&content())).unwrap();
        assert_eq!(
            a, b,
            "an unchanged repository must produce an identical index"
        );
    }

    #[test]
    fn entries_carry_a_link_the_browser_can_follow() {
        let index = build(&content());
        assert_eq!(index.entries[0].href, "/learn/ownership/published");
        assert_eq!(index.entries[0].kind, EntryKind::Lesson);
    }

    #[test]
    fn an_empty_repository_produces_a_valid_empty_index() {
        let index = build(&Content::default());
        assert_eq!(index.format_version, INDEX_FORMAT_VERSION);
        assert!(index.entries.is_empty());
        assert!(serde_json::to_string(&index).is_ok());
    }
}
