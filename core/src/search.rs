use std::cmp::Ordering;

use crate::{AppState, Entity};

pub struct SearchEngine;

impl SearchEngine {
    /// Matches a precomputed, already-lowercased haystack against an
    /// already-lowercased query. Both sides are lowercased once by the caller
    /// (haystack at load time, query once per keystroke) to keep the filter
    /// hot path allocation-free.
    pub fn matches(haystack: &str, query_lower: &str) -> bool {
        query_lower.is_empty() || haystack.contains(query_lower)
    }

    pub fn compare(a: &Entity, b: &Entity, app_state: &AppState) -> Ordering {
        let score_a = app_state.get_score(a);
        let score_b = app_state.get_score(b);

        let score_ordering = score_b.cmp(&score_a);

        if score_ordering == Ordering::Equal {
            return a.name().to_lowercase().cmp(&b.name().to_lowercase());
        }

        score_ordering
    }
}
