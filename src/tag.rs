use std::{fmt, ops::Deref, str::FromStr};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum TagError {
    #[error("tag is empty")]
    Empty,
    #[error("tag too long")]
    TooLong,
    #[error("invalid character")]
    InvalidChar,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct Tag(String);

impl Tag {
    pub fn new(s: String) -> Result<Self, TagError> {
        Self::validate(&s)?;
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn validate(s: &str) -> Result<(), TagError> {
        if s.is_empty() {
            return Err(TagError::Empty);
        }
        if s.len() > 64 {
            return Err(TagError::TooLong);
        }
        if !s.chars().next().unwrap().is_ascii_alphanumeric() {
            return Err(TagError::InvalidChar);
        }
        if !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            return Err(TagError::InvalidChar);
        }
        Ok(())
    }
}

impl Deref for Tag {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for Tag {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for Tag {
    type Err = TagError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_owned())
    }
}

impl TryFrom<&str> for Tag {
    type Error = TagError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl TryFrom<String> for Tag {
    type Error = TagError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}
