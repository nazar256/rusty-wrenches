use std::{error::Error, ffi::OsStr, fs, io::Error, path::{Iter, Path, PathBuf}};
use futures::stream::{Stream, StreamExt};
use std::pin::Pin;

// fn walk_dirs(dir: &Path) -> Pin<Box<dyn Stream<Item = Result<fs::DirEntry, std::io::Error>> + '_>> {
//     Box::pin(try_stream! {
//         let mut read_dir = fs::read_dir(dir).await?;
//         while let Some(entry) = read_dir.next_entry().await? {
//             let path = entry.path();
//             if path.is_dir() {
//                 log::debug!("Found directory: {:?}", path);
//                 yield entry;
//                 let mut subdir_stream = walk_dirs(&path);
//                 while let Some(subentry) = subdir_stream.next().await {
//                     yield subentry?;
//                 }
//             }
//         }
//     })
// }

// fn walk_dirs_simple(dir: &Path) -> Result<Iter<Path>, std::io::Error> {
//     let paths = fs::read_dir(dir)?
//     .filter(|r| r.is_ok())
//     .map(|r| r.unwrap())
//     .filter(|r| r.file_type().unwrap().is_dir())
//     .map(|r| r.path());
//     Ok(read_dir)
// }

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

fn count_nested_dirs(dir: &Path, skip_name_match: bool) -> Result<usize, std::io::Error> {
    let parent_name = dir.file_name().unwrap_or_else(|| {
        log::error!("Failed to get file name for {:?}", dir);
        return OsStr::new("");
    });
    let read_dir = fs::read_dir(dir)?;
    let count = read_dir
    .filter_map(|r| r.ok())
    .filter(|r| !skip_name_match || {
        r.path().file_name().unwrap() == parent_name
    })
    .count();
    Ok(count)
}


pub fn fix_nested_directories(
    path: &Path,
    skip_name_match: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Starting to fix nested directories");

    DirWalker::new(path)?
    .filter(|path| count_nested_dirs(path, skip_name_match).unwrap() == 1)
    .try_for_each(|r| {
        unnest(&r)
    });
    
    // while let Some(entry) = stream.next().await {
    //     let dir = entry?;
    //     log::debug!("Checking if {:?} has only one nested directory", dir.path());
    //     let entries = ReadDirStream::new(fs::read_dir(dir.path()).await?)
    //     .map(|r| r.unwrap())
    //     .map(|r| r.path()) // This is safe, since we only have the Ok variants
    //     .filter(|r| r.is_dir()) // Filter out non-folders
    //     .collect::<Vec<_>>();
    //     if entries.len() == 1 {
    //         let nested_dir = entries.first().unwrap().path();
    //         log::debug!("Found nested directory: {:?}", nested_dir);
    //     }
    //     if skip_name_match || dir.file_name() == dir.path().parent().unwrap().file_name().unwrap() {
    //         move_contents_to_parent(&dir.path()).await?;
    //     }
    // }
    Ok(())
}

fn unnest(from_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    //TODO add guard that checks that from_dir is a directory
    log::info!("Moving contents to {:?}", from_dir);
    fs::read_dir(from_dir)
    .and_then(|dir| {
        dir.into_iter()
        .filter_map(|e| e.ok())
        .try_for_each(|entry| {
            log::info!("Moving {:?} to {:?}", entry.path(), from_dir.parent().unwrap().join(entry.file_name()));
            fs::rename(&entry.path(), from_dir.parent().unwrap().join(entry.file_name()))?;
            Ok(())
            // let entry_path = d.path();
            // let parent = from_dir.parent().ok_or("Failed to get parent path")?;
            // let file_name = entry_path.file_name().ok_or("Failed to get file name")?;
            // log::debug!("Moving {:?} to {:?}", entry_path, parent.join(file_name));
            // fs::rename(&entry_path, parent.join(file_name)).await?;
        })
    })
       .
    // let mut read_dir = fs::read_dir(from_dir).await?;
    // while let Some(entry) = fs::read_dir(from_dir).await?.next_entry().await? {
    //     let entry_path = entry.path();
    //     let parent = from_dir.parent().ok_or("Failed to get parent path")?;
    //     let file_name = entry_path.file_name().ok_or("Failed to get file name")?;
    //     log::debug!("Moving {:?} to {:?}", entry_path, parent.join(file_name));
    //     fs::rename(&entry_path, parent.join(file_name)).await?;
    // }
    // // remove directory if it's now empty
    // if fs::read_dir(from_dir).await?.next_entry().await?.is_none() {
    //     fs::remove_dir(from_dir).await?;
    // }
    Ok(())
} 

#[cfg(test)]
mod tests {
    use crate::cli::init_logging;

    use super::*;
    use std::env::temp_dir;
    use rand::random;
    use tokio::fs;

    async fn setup_test_dir(nested_name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let temp = temp_dir().join(format!("test_{}", random::<u32>()));
        let nested = temp.join("nested");
        let subdir = nested.join(nested_name);
        // let _ = subdir.join("another_nested");
        let file = subdir.join("file.txt");

        fs::create_dir_all(&subdir).await.unwrap();
        fs::write(&file, "test content").await.unwrap();
        
        (temp, file)
    }

    struct TestCase {
        name: &'static str,
        skip_name_match: bool,
        nested_dir_name: &'static str,
        expect_moved: bool,
    }

    #[tokio::test]
    async fn test_fix_nested_directories() {
        init_logging();
        
        let test_cases = vec![
            // TestCase {
            //     name: "should not move files when skip_name_match is false and names don't match",
            //     skip_name_match: false,
            //     nested_dir_name: "subdir",
            //     expect_moved: false,
            // },
            TestCase {
                name: "should move files when skip_name_match is true",
                skip_name_match: true,
                nested_dir_name: "subdir",
                expect_moved: true,
            },
            // TestCase {
            //     name: "should move files when names match even if skip_name_match is false",
            //     skip_name_match: false,
            //     nested_dir_name: "nested",
            //     expect_moved: true,
            // },
        ];

        for tc in test_cases {
            let (temp_dir, original_file) = setup_test_dir(tc.nested_dir_name).await;
            
            fix_nested_directories(&temp_dir, tc.skip_name_match).await.unwrap();
            
            let expected_path = if tc.expect_moved {
                temp_dir.join("nested").join("file.txt")
            } else {
                original_file.clone()
            };

            assert!(
                fs::try_exists(&expected_path).await.unwrap(),
                "{}: file should exist at {:?}",
                tc.name,
                expected_path
            );

            if tc.expect_moved {
                assert!(
                    !fs::try_exists(&original_file).await.unwrap(),
                    "{}: original file should not exist at {:?}",
                    tc.name,
                    original_file
                );
                assert!(
                    !fs::try_exists(&temp_dir.join("nested")).await.unwrap(),
                    "{}: nested directory should not exist at {:?}",
                    tc.name,
                    temp_dir.join("nested")
                );
            }
            

            fs::remove_dir_all(temp_dir).await.unwrap();
        }
    }
}
