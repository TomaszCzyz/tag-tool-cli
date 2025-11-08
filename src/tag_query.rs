use crate::tag::{Tag, TagError};
use std::fmt::{Debug, Formatter};
use std::str::FromStr;

/// Examples
/// - `tag1 tag2 tag3` - items can have tag1 or tag2 or tag3
/// - `tag1 -tag2 tag3` - items can have tag1 or tag3 and **cannot** have tag2
/// - `tag1 -tag2 +tag3` - items can have tag1 and **cannot** have tag2 and **must** have tag3
///
/// The order does not matter. For example, `tag1 tag2 -tag3` is the same as `tag2 -tag3 tag1`.
#[derive(Clone)]
pub struct TagQuery {
    optional_tags: Vec<Tag>,
    included_tags: Vec<Tag>,
    excluded_tags: Vec<Tag>,
}

impl Debug for TagQuery {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();
        parts.extend(self.optional_tags.iter().map(|t| t.to_string()));
        parts.extend(self.included_tags.iter().map(|t| format!("+{}", t)));
        parts.extend(self.excluded_tags.iter().map(|t| format!("-{}", t)));
        write!(f, "{}", parts.join(" "))
    }
}

impl TryFrom<&str> for TagQuery {
    type Error = TagError;

    fn try_from(query: &str) -> Result<Self, Self::Error> {
        let mut optional_tags = Vec::new();
        let mut included_tags = Vec::new();
        let mut excluded_tags = Vec::new();

        for query_part in query.split_whitespace() {
            let mut chars = query_part.chars();
            match chars.next() {
                Some('-') => {
                    let tag = Tag::try_from(chars.as_str())?;
                    excluded_tags.push(tag);
                }
                Some('+') => {
                    let tag = Tag::try_from(chars.as_str())?;
                    included_tags.push(tag.clone());
                    optional_tags.push(tag);
                }
                Some(c) if c.is_ascii_alphanumeric() => {
                    let tag = Tag::try_from(query_part)?;
                    optional_tags.push(tag);
                }
                _ => return Err(TagError::InvalidChar),
            }
        }

        Ok(Self {
            optional_tags,
            included_tags,
            excluded_tags,
        })
    }
}

impl FromStr for TagQuery {
    type Err = TagError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        TagQuery::try_from(s)
    }
}

impl TagQuery {
    pub fn optional_tags(&self) -> &[Tag] {
        &self.optional_tags
    }

    pub fn included_tags(&self) -> &[Tag] {
        &self.included_tags
    }

    pub fn excluded_tags(&self) -> &[Tag] {
        &self.excluded_tags
    }
}
