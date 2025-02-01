use std::{
    error::Error,
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
};

/// Simple DFS directory iterator.
struct DirWalker {
    stack: Vec<PathBuf>,
}

impl DirWalker {
    /// Create a new `DirWalker` starting at `root` (if `root` is a directory).
    fn new(root: &Path) -> std::io::Result<Self> {
        let mut stack = Vec::new();

        // Only push `root` if it's an actual directory.
        if root.is_dir() {
            stack.push(root.to_path_buf());
        }

        Ok(Self { stack })
    }
}

impl Iterator for DirWalker {
    type Item = PathBuf;

    fn next(&mut self) -> Option<Self::Item> {
        // Pop the top directory from the stack (DFS).
        let dir = self.stack.pop()?;

        // Read its entries, push subdirectories to the stack.
        if let Ok(entries) = fs::read_dir(&dir) {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.is_dir())
                .for_each(|path| self.stack.push(path));
        }

        // Yield the current directory.
        Some(dir)
    }
}

fn count_nested_dirs(
    dir: &Path,
    skip_name_match: bool,
) -> Result<usize, Box<dyn std::error::Error>> {
    let parent_name = dir.file_name().unwrap_or_else(|| {
        log::error!("Failed to get file name for {:?}", dir);
        OsStr::new("")
    });
    let count = list_nested_items(dir)?
        .filter(|r| r.is_dir())
        .filter(|r| {
            skip_name_match || {
                let name_matches = r.as_path().file_name().unwrap() == parent_name;
                log::debug!("Directory: {:?}, Name matches: {:?}", r, name_matches);
                name_matches
            }
        })
        .count();
    log::debug!("Counted {} nested directories in {:?}", count, dir);
    Ok(count)
}

// Returns an iterator over the nested items in the directory.
fn list_nested_items(
    dir: &Path,
) -> Result<impl Iterator<Item = PathBuf>, Box<dyn std::error::Error>> {
    let items = fs::read_dir(dir)?.filter_map(|r| r.ok()).map(|r| r.path());
    Ok(items)
}

pub fn fix_nested_directories(
    path: &Path,
    skip_name_match: bool,
    dry_run: bool,
) -> Result<(), Box<dyn Error>> {
    log::info!("Starting to fix redundant nested directories");

    DirWalker::new(path)?
        .filter(|path| count_nested_dirs(path, skip_name_match).unwrap_or(0) == 1)
        .try_for_each(|r| {
            // from nested dir to current
            unnest(&r, dry_run)
        })
}

fn unnest(parent: &Path, dry_run: bool) -> Result<(), Box<dyn Error>> {
    //TODO add guard that checks that from_dir is a directory
    let nested_dir = list_nested_items(parent)?.next().ok_or(io::Error::new(
        io::ErrorKind::Other,
        "Failed to get nested directory",
    ))?;
    log::info!("Moving contents from {:?} to {:?}", nested_dir, parent);
    list_nested_items(nested_dir.as_path())?.try_for_each(|src| {
        let src_filename = src.file_name().ok_or(io::Error::new(
            io::ErrorKind::Other,
            "Failed to get file name",
        ))?;
        let dst = parent.join(src_filename);
        log::info!("Moving {:?} to {:?}", src, dst);
        if !dry_run {
            fs::rename(&src, dst)?;
        }
        io::Result::Ok(())
    })?;
    if list_nested_items(nested_dir.as_path())?.count() == 0 {
        log::info!("Removing empty nested directory {:?}", nested_dir);
        if !dry_run {
            fs::remove_dir(nested_dir.as_path())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::cli::init_logging;

    use super::*;
    use rand::random;
    use std::env::temp_dir;

    const CONTENTS_DIR_NAME: &str = "contents";

    // Returns a tuple of the temp directory and the file that was created
    // Creates the following directory structure:
    // temp_dir/contents
    // temp_dir/another_dir
    // temp_dir/contents/nested_name
    // temp_dir/contents/nested_name/file.txt
    fn setup_test_tree(nested_name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let temp = temp_dir().join(format!("test_{}", random::<u32>()));
        let contents = temp.join(CONTENTS_DIR_NAME);
        let nested = contents.join(nested_name);
        let another_dir = temp.join("another_dir");
        let file = nested.join("file.txt");

        fs::create_dir_all(&nested).unwrap();
        fs::create_dir_all(&another_dir).unwrap();
        fs::write(&file, "test content").unwrap();

        (temp, file)
    }

    struct TestCase {
        name: &'static str,
        skip_name_match: bool,
        nested_dir_name: &'static str,
        expect_moved: bool,
        dry_run: bool,
    }

    #[test]
    fn test_fix_nested_directories() {
        init_logging(log::LevelFilter::Debug);

        let test_cases = vec![
            TestCase {
                name: "should not move files when skip_name_match is false and names don't match",
                skip_name_match: false,
                nested_dir_name: "nested",
                expect_moved: false,
                dry_run: false,
            },
            TestCase {
                name: "should move files when skip_name_match is true",
                skip_name_match: true,
                nested_dir_name: "nested",
                expect_moved: true,
                dry_run: false,
            },
            TestCase {
                name: "should move files when names match even if skip_name_match is false",
                skip_name_match: false,
                nested_dir_name: "contents",
                expect_moved: true,
                dry_run: false,
            },
            TestCase {
                name: "should not move files when dry_run is true",
                skip_name_match: false,
                nested_dir_name: "nested",
                expect_moved: false,
                dry_run: true,
            },
        ];

        for tc in test_cases {
            let (temp_dir, original_file) = setup_test_tree(tc.nested_dir_name);

            fix_nested_directories(&temp_dir, tc.skip_name_match, tc.dry_run).unwrap();

            let expected_path = if tc.expect_moved {
                temp_dir.join(CONTENTS_DIR_NAME).join("file.txt")
            } else {
                original_file.clone()
            };

            assert!(
                fs::exists(&expected_path).unwrap(),
                "{}: file should exist at {:?}",
                tc.name,
                expected_path
            );

            if tc.expect_moved {
                assert!(
                    !fs::exists(&original_file).unwrap(),
                    "{}: original file should not exist at {:?}",
                    tc.name,
                    original_file
                );
                assert!(
                    !fs::exists(&temp_dir.join(CONTENTS_DIR_NAME).join(tc.nested_dir_name))
                        .unwrap(),
                    "{}: nested directory should not exist at {:?}",
                    tc.name,
                    temp_dir.join(CONTENTS_DIR_NAME).join(tc.nested_dir_name)
                );
            }

            fs::remove_dir_all(temp_dir).unwrap();
        }
    }
}
