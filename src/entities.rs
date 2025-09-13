use blake3::{Hash, OUT_LEN};
use sqlx::types::chrono::{DateTime, Utc};
use sqlx::{Database, Decode, FromRow};
use std::fmt::{Display, Formatter};

#[derive(FromRow, Debug)]
#[sqlx(rename_all = "PascalCase")]
#[derive(PartialEq)]
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

#[derive(FromRow, Debug)]
#[sqlx(rename_all = "PascalCase")]
pub struct File {
    pub(crate) id: i64,
    pub(crate) path: String,
    pub(crate) hash: Box<[u8]>,
    pub(crate) created_at: DateTime<Utc>,

    #[sqlx(skip)]
    pub(crate) tags: Vec<Tag>,
}

impl Display for File {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path)
    }
}
