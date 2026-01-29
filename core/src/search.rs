use std::cmp::Ordering;

use crate::{AppState, Entity};

pub struct SearchEngine;

impl SearchEngine {
    pub fn matches(entity: &Entity, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }

        let query_lower = query.to_lowercase();

        // Check name
        if entity.name().to_lowercase().contains(&query_lower) {
            return true;
        }

        // Check description
        if let Some(desc) = entity.description()
            && desc.to_lowercase().contains(&query_lower)
        {
            return true;
        }

        false
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
