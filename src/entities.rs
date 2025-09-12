use sqlx::FromRow;
use sqlx::types::chrono::{DateTime, Utc};
use std::fmt::{Display, Formatter};

#[derive(FromRow, Debug)]
#[sqlx(rename_all = "PascalCase")]
pub struct Tag {
    pub(crate) id: i64,
    pub(crate) text: String,
    pub(crate) created_at: DateTime<Utc>,
}

impl Display for Tag {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text)
    }
}
