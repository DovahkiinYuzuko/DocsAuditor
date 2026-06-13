use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

pub struct FileLocker {
    lock_file_path: PathBuf,
}

impl FileLocker {
    pub fn try_lock(target_path: &Path) -> Option<FileLocker> {
        let lock_file_path = target_path.with_extension("lock");
        
        // create_new(true) は、ファイルが既に存在する場合はエラーとなるためアトミックな作成が可能
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_file_path)
        {
            Ok(_) => Some(FileLocker { lock_file_path }),
            Err(_) => None,
        }
    }
}

impl Drop for FileLocker {
    fn drop(&mut self) {
        if self.lock_file_path.exists() {
            let _ = std::fs::remove_file(&self.lock_file_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_lock_success_and_auto_release() {
        let test_file = Path::new("test_file_for_lock.txt");
        let lock_file = test_file.with_extension("lock");

        // ロック前は存在しない
        assert!(!lock_file.exists());

        {
            let locker = FileLocker::try_lock(test_file);
            assert!(locker.is_some());
            assert!(lock_file.exists());

            // 重複ロックの試みは失敗する
            let second_locker = FileLocker::try_lock(test_file);
            assert!(second_locker.is_none());
        } // ここで locker がドロップされ自動的にアンロックされる

        // アンロック後は存在しない
        assert!(!lock_file.exists());
    }
}
