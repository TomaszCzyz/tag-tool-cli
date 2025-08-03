use directories::ProjectDirs;
use std::borrow::Cow;
use std::collections::HashSet;
use std::collections::hash_set::Iter;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug)]
pub struct TagsStorage {
    tags_data_path: PathBuf,

    /// All user defined tags.
    tags: HashSet<Cow<'static, str>>,
}

impl Default for TagsStorage {
    fn default() -> Self {
        if let Some(proj_dirs) = ProjectDirs::from("com", "example", "tagtool") {
            let data_path = proj_dirs.data_dir().join("tags.txt");

            if !data_path.exists() {
                fs::create_dir_all(proj_dirs.data_dir()).unwrap();
                fs::write(&data_path, "").unwrap();
            }

            let tags = fs::read_to_string(&data_path)
                .unwrap()
                .lines()
                .map(|line| Cow::Owned(line.to_string()))
                .collect::<HashSet<_>>();

            Self {
                tags_data_path: data_path,
                tags,
            }
        } else {
            panic!("Failed to get project directories");
        }
    }
}

impl TagsStorage {
    pub fn add(&mut self, tag: Cow<'static, str>) {
        if self.tags.contains(&tag) {
            return;
        }

        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(&self.tags_data_path)
            .unwrap();

        file.write_all(format!("{}\n", tag).as_bytes()).unwrap();
        self.tags.insert(tag);
    }

    pub fn list(&self) -> Iter<'_, Cow<'static, str>> {
        self.tags.iter()
    }
}
