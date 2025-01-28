use std::{error::Error, ffi::OsStr, fs, io, path::{Path, PathBuf}};

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
            for entry_res in entries {
                if let Ok(entry) = entry_res {
                    let path = entry.path();
                    if path.is_dir() {
                        self.stack.push(path);
                    }
                }
            }
        }

        // Yield the current directory.
        Some(dir)
    }
}

fn count_nested_dirs(dir: &Path, skip_name_match: bool) -> Result<usize, Box<dyn std::error::Error>> {
    let parent_name = dir.file_name().unwrap_or_else(|| {
        log::error!("Failed to get file name for {:?}", dir);
        return OsStr::new("");
    });
    let count = list_nested_items(dir)?
    .filter(|r| r.is_dir())
    .filter(|r| !skip_name_match || {
        r.as_path().file_name().unwrap() == parent_name
    })
    .count();
    Ok(count)
}

// Returns an iterator over the nested items in the directory.
fn list_nested_items(dir: &Path) -> Result<impl Iterator<Item = PathBuf>, Box<dyn std::error::Error>> {
    let items = fs::read_dir(dir)?
    .filter_map(|r| r.ok())
    .map(|r| r.path());
    Ok(items)
}


pub fn fix_nested_directories(
    path: &Path,
    skip_name_match: bool,
) -> Result<(), Box<dyn Error>> {
    log::info!("Starting to fix nested directories");

    DirWalker::new(path)?
    .filter(|path| count_nested_dirs(path, skip_name_match).unwrap_or(0) == 1)
    .try_for_each(|r| {
        unnest(&r)
    })
}

fn unnest(from_dir: &Path) -> Result<(), Box<dyn Error>> {
    //TODO add guard that checks that from_dir is a directory
    let parent = from_dir.parent()
    .ok_or(io::Error::new(io::ErrorKind::Other, "Failed to get parent directory"))?;
    log::info!("Moving contents to {:?}", from_dir);
    list_nested_items(from_dir)?
    .try_for_each(|src| {
        let src_filename = src.file_name()
            .ok_or(io::Error::new(io::ErrorKind::Other, "Failed to get file name"))?;
        let dst = parent.join(src_filename);
        log::info!("Moving {:?} to {:?}", src, dst);

        fs::rename(&src, dst)
    })?;
    Ok(())
} 

#[cfg(test)]
mod tests {
    use crate::cli::init_logging;

    use super::*;
    use std::env::temp_dir;
    use rand::random;

    fn setup_test_dir(nested_name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let temp = temp_dir().join(format!("test_{}", random::<u32>()));
        let nested = temp.join("nested");
        let subdir = nested.join(nested_name);
        // let _ = subdir.join("another_nested");
        let file = subdir.join("file.txt");

        fs::create_dir_all(&subdir).unwrap();
        fs::write(&file, "test content").unwrap();
        
        (temp, file)
    }

    struct TestCase {
        name: &'static str,
        skip_name_match: bool,
        nested_dir_name: &'static str,
        expect_moved: bool,
    }

    #[test]
    fn test_fix_nested_directories() {
        init_logging();
        
        let test_cases = vec![
            TestCase {
                name: "should not move files when skip_name_match is false and names don't match",
                skip_name_match: false,
                nested_dir_name: "subdir",
                expect_moved: false,
            },
            TestCase {
                name: "should move files when skip_name_match is true",
                skip_name_match: true,
                nested_dir_name: "subdir",
                expect_moved: true,
            },
            TestCase {
                name: "should move files when names match even if skip_name_match is false",
                skip_name_match: false,
                nested_dir_name: "nested",
                expect_moved: true,
            },
        ];

        for tc in test_cases {
            let (temp_dir, original_file) = setup_test_dir(tc.nested_dir_name);
            
            fix_nested_directories(&temp_dir, tc.skip_name_match).unwrap();
            
            let expected_path = if tc.expect_moved {
                temp_dir.join("nested").join("file.txt")
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
                    !fs::exists(&temp_dir.join("nested")).unwrap(),
                    "{}: nested directory should not exist at {:?}",
                    tc.name,
                    temp_dir.join("nested")
                );
            }
            

            fs::remove_dir_all(temp_dir).unwrap();
        }
    }
}
