//! 跨进程文件锁
//!
//! 基于 OS 文件锁（Windows LockFileEx / Unix flock），进程退出时自动释放；
//! 锁文件保留不删，避免删除-重建之间出现两个实例同时持锁的竞态

use crate::errors::LauncherError;
use std::fs::{File, OpenOptions, TryLockError};
use std::path::Path;
use std::time::{Duration, Instant};

const RETRY_INTERVAL: Duration = Duration::from_millis(25);

#[derive(Debug)]
pub struct FileLock {
    file: File,
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

fn open_lock_file(path: &Path) -> Result<File, LauncherError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|e| LauncherError::Custom(format!("打开锁文件 {} 失败: {}", path.display(), e)))
}

fn lock_error(path: &Path, err: TryLockError) -> LauncherError {
    LauncherError::Custom(format!("锁定 {} 失败: {}", path.display(), err))
}

fn busy_error(path: &Path, timeout: Duration) -> LauncherError {
    LauncherError::Busy(format!(
        "{}（等待 {:.1} 秒超时）",
        path.display(),
        timeout.as_secs_f32()
    ))
}

fn try_lock(file: &File, path: &Path, shared: bool) -> Result<bool, LauncherError> {
    let result = if shared {
        file.try_lock_shared()
    } else {
        file.try_lock()
    };
    match result {
        Ok(()) => Ok(true),
        Err(TryLockError::WouldBlock) => Ok(false),
        Err(e) => Err(lock_error(path, e)),
    }
}

fn acquire_with(path: &Path, timeout: Duration, shared: bool) -> Result<FileLock, LauncherError> {
    let file = open_lock_file(path)?;
    let deadline = Instant::now() + timeout;
    loop {
        if try_lock(&file, path, shared)? {
            return Ok(FileLock { file });
        }
        if Instant::now() >= deadline {
            return Err(busy_error(path, timeout));
        }
        std::thread::sleep(RETRY_INTERVAL);
    }
}

async fn acquire_async_with(
    path: &Path,
    timeout: Duration,
    shared: bool,
) -> Result<FileLock, LauncherError> {
    let file = open_lock_file(path)?;
    let deadline = Instant::now() + timeout;
    loop {
        if try_lock(&file, path, shared)? {
            return Ok(FileLock { file });
        }
        if Instant::now() >= deadline {
            return Err(busy_error(path, timeout));
        }
        tokio::time::sleep(RETRY_INTERVAL).await;
    }
}

/// 同步获取排他锁；配置写入这类短临界区用它
pub fn acquire(path: &Path, timeout: Duration) -> Result<FileLock, LauncherError> {
    acquire_with(path, timeout, false)
}

/// 异步获取排他锁；下载、token 刷新这类可能等较久的场景用它，避免阻塞 worker
pub async fn acquire_async(path: &Path, timeout: Duration) -> Result<FileLock, LauncherError> {
    acquire_async_with(path, timeout, false).await
}

/// 共享锁：多个持有者可以同时存在，排他锁必须等它们全部释放
pub async fn acquire_shared_async(
    path: &Path,
    timeout: Duration,
) -> Result<FileLock, LauncherError> {
    acquire_async_with(path, timeout, true).await
}

/// 同步共享锁；测试和需要短等待的场景用它
pub fn acquire_shared(path: &Path, timeout: Duration) -> Result<FileLock, LauncherError> {
    acquire_with(path, timeout, true)
}

#[cfg(test)]
mod tests {
    use super::{acquire, acquire_async, acquire_shared};
    use std::time::Duration;

    #[test]
    fn test_multiple_shared_locks_can_coexist() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("run.lock");

        let first = acquire_shared(&path, Duration::from_millis(50)).unwrap();
        let second = acquire_shared(&path, Duration::from_millis(50)).unwrap();

        // 运行中的实例不能删除/重命名
        assert!(acquire(&path, Duration::from_millis(50)).is_err());

        drop(first);
        drop(second);
        assert!(acquire(&path, Duration::from_millis(50)).is_ok());
    }

    #[test]
    fn test_shared_lock_cannot_bypass_exclusive_holder() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("run.lock");

        let exclusive = acquire(&path, Duration::from_millis(50)).unwrap();
        assert!(acquire_shared(&path, Duration::from_millis(50)).is_err());

        drop(exclusive);
        assert!(acquire_shared(&path, Duration::from_millis(50)).is_ok());
    }

    #[test]
    fn test_exclusive_lock_released_on_drop() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("test.lock");

        let first = acquire(&path, Duration::from_millis(50)).unwrap();
        let err = acquire(&path, Duration::from_millis(50)).unwrap_err();
        assert!(err.to_string().contains("另一个启动器实例"), "{err}");

        drop(first);
        assert!(acquire(&path, Duration::from_millis(50)).is_ok());
    }

    #[tokio::test]
    async fn test_async_lock_waits_for_release() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("test.lock");

        let first = acquire(&path, Duration::from_millis(50)).unwrap();
        assert!(
            acquire_async(&path, Duration::from_millis(50))
                .await
                .is_err()
        );

        drop(first);
        assert!(
            acquire_async(&path, Duration::from_millis(50))
                .await
                .is_ok()
        );
    }
}
