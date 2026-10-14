//! Multi-field input.

/// Pre-split address fields. All optional; empty strings are treated as absent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Fields {
    pub line1: Option<String>,
    pub line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
}

impl Fields {
    /// Join the non-empty fields into a single comma-separated line.
    pub(crate) fn joined(&self) -> String {
        [&self.line1, &self.line2, &self.city, &self.state, &self.zip]
            .iter()
            .filter_map(|f| f.as_deref().map(str::trim))
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(", ")
    }
}
