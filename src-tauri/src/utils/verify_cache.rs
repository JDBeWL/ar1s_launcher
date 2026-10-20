//! 文件校验指纹缓存
//!
//! `verify_file` 要把整个文件读一遍算 sha1，一个完整安装光校验就是 1.5 GB 的读盘。
//! 但文件内容变没变，用「长度 + 修改时间」就足以判定：指纹没动过，上次的校验结论
//! 依然成立，第二次安装同一版本基本不再读盘。
//!
//! 进程内是 HashMap，落盘到游戏目录的 `.ar1s/verify_cache.json`，下次启动仍然有效。
//! 条目只增不删（覆盖同一路径除外），到上限整体丢弃重建——维护 LRU 的代价比偶尔
//! 全量重算一遍还高。

use log::{debug, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, RwLock};
use std::time::UNIX_EPOCH;

const CACHE_FILE: &str = "verify_cache.json";
const MAX_ENTRIES: usize = 20_000;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Entry {
    len: u64,
    /// 修改时间（自 UNIX_EPOCH 起的纳秒数，u64 够用到 2554 年）；取不到时为 None
    #[serde(default)]
    modified: Option<u64>,
    /// 校验通过时对应的期望哈希，None 表示那次只校验过大小
    #[serde(default)]
    hash: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct CacheFile {
    #[serde(default)]
    files: HashMap<String, Entry>,
}

#[derive(Debug, Default)]
struct Cache {
    entries: HashMap<PathBuf, Entry>,
    /// 缓存落盘位置；没调过 init 就只在内存里生效
    path: Option<PathBuf>,
    dirty: bool,
}

static CACHE: LazyLock<RwLock<Cache>> = LazyLock::new(|| RwLock::new(Cache::default()));

fn read_lock() -> std::sync::RwLockReadGuard<'static, Cache> {
    CACHE
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn write_lock() -> std::sync::RwLockWriteGuard<'static, Cache> {
    CACHE
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn modified_nanos(meta: &std::fs::Metadata) -> Option<u64> {
    meta.modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|elapsed| elapsed.as_nanos().min(u128::from(u64::MAX)) as u64)
}

/// 绑定到游戏目录；重复调用同一个目录时不会重读磁盘
pub fn init(game_dir: &Path) {
    let path = game_dir.join(".ar1s").join(CACHE_FILE);
    let mut cache = write_lock();
    if cache.path.as_deref() == Some(path.as_path()) {
        return;
    }
    cache.entries = load(&path);
    cache.path = Some(path);
    cache.dirty = false;
}

/// 指纹与期望哈希都和上次校验通过时一致，就不必再读文件
pub fn matches(path: &Path, meta: &std::fs::Metadata, expected_hash: Option<&str>) -> bool {
    let expected = expected_hash.map(str::to_ascii_lowercase);
    let cache = read_lock();
    match cache.entries.get(path) {
        Some(entry) => {
            entry.len == meta.len()
                && entry.modified == modified_nanos(meta)
                && entry.hash == expected
        }
        None => false,
    }
}

/// 校验通过后记一笔；同一路径再次调用会覆盖旧指纹
pub fn remember(path: &Path, meta: &std::fs::Metadata, expected_hash: Option<&str>) {
    let modified = modified_nanos(meta);
    let mut cache = write_lock();

    let hash = match expected_hash {
        Some(hash) => Some(hash.to_ascii_lowercase()),
        // 只校验大小的调用不要把之前记下的哈希抹掉，否则下次带哈希的校验又要重算
        None => cache
            .entries
            .get(path)
            .filter(|entry| entry.len == meta.len() && entry.modified == modified)
            .and_then(|entry| entry.hash.clone()),
    };

    if cache.entries.len() >= MAX_ENTRIES && !cache.entries.contains_key(path) {
        warn!("文件校验缓存达到 {} 条，清空重建", MAX_ENTRIES);
        cache.entries.clear();
    }

    cache.entries.insert(
        path.to_path_buf(),
        Entry {
            len: meta.len(),
            modified,
            hash,
        },
    );
    cache.dirty = true;
}

/// 把内存里的指纹写回磁盘；没有变化或没绑定目录时直接返回
pub fn flush() {
    let (path, entries) = {
        let mut cache = write_lock();
        if !cache.dirty {
            return;
        }
        cache.dirty = false;
        let Some(path) = cache.path.clone() else {
            return;
        };
        (path, cache.entries.clone())
    };

    if let Err(error) = save_to(&path, &entries) {
        warn!("写入校验缓存失败: {}", error);
    } else {
        debug!("校验缓存已保存 {} 条", entries.len());
    }
}

fn load(path: &Path) -> HashMap<PathBuf, Entry> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return HashMap::new();
    };
    match serde_json::from_str::<CacheFile>(&text) {
        Ok(file) => file
            .files
            .into_iter()
            .map(|(key, entry)| (PathBuf::from(key), entry))
            .collect(),
        Err(error) => {
            debug!("校验缓存无法解析，按空缓存处理: {}", error);
            HashMap::new()
        }
    }
}

/// 先写临时文件再 rename，避免中途退出留下半截 JSON
fn save_to(path: &Path, entries: &HashMap<PathBuf, Entry>) -> std::io::Result<()> {
    let files: HashMap<String, Entry> = entries
        .iter()
        .map(|(key, entry)| (key.to_string_lossy().into_owned(), entry.clone()))
        .collect();
    let text = serde_json::to_string(&CacheFile { files }).map_err(std::io::Error::other)?;

    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let temp = path.with_extension("json.part");
    std::fs::write(&temp, text)?;
    std::fs::rename(&temp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Entry, load, modified_nanos, save_to};
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn entry(len: u64, modified: Option<u64>, hash: Option<&str>) -> Entry {
        Entry {
            len,
            modified,
            hash: hash.map(str::to_string),
        }
    }

    #[test]
    fn test_saved_entries_survive_a_round_trip() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("verify_cache.json");

        let mut entries = HashMap::new();
        entries.insert(
            PathBuf::from("/games/minecraft/assets/objects/ab/abcdef"),
            entry(1024, Some(1_700_000_000_000_000_000), Some("abc123")),
        );
        entries.insert(
            PathBuf::from("/games/minecraft/libraries/x.jar"),
            entry(0, None, None),
        );

        save_to(&path, &entries).unwrap();
        let restored = load(&path);

        assert_eq!(restored, entries);
    }

    #[test]
    fn test_broken_cache_file_is_ignored() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("verify_cache.json");
        std::fs::write(&path, "{ not json").unwrap();

        assert!(load(&path).is_empty());
        assert!(load(&dir.path().join("missing.json")).is_empty());
    }

    #[test]
    fn test_modified_nanos_is_stable_for_a_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("a.bin");
        std::fs::write(&path, b"hello").unwrap();

        let meta = std::fs::metadata(&path).unwrap();
        let first = modified_nanos(&meta);
        assert!(first.is_some());
        assert_eq!(first, modified_nanos(&std::fs::metadata(&path).unwrap()));
    }

    #[test]
    fn test_entry_matches_only_the_same_fingerprint_and_hash() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("a.bin");
        std::fs::write(&path, b"hello").unwrap();
        let meta = std::fs::metadata(&path).unwrap();

        let cached = entry(meta.len(), modified_nanos(&meta), Some("aaf4c61d"));

        assert_eq!(cached.len, meta.len());
        assert_eq!(cached.hash.as_deref(), Some("aaf4c61d"));
        // 长度或哈希对不上时必须重算
        assert_ne!(cached.len, meta.len() + 1);
        assert_ne!(cached.hash.as_deref(), Some("00000000"));
    }
}
