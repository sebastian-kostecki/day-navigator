//! The two spaces an item can belong to. Neutral: no store imports.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Space {
    Work,
    Private,
}

impl std::fmt::Display for Space {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Space::Work => "work",
            Space::Private => "private",
        })
    }
}
