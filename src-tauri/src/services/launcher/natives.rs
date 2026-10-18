//! Natives 库解压逻辑
//!
//! 按 OS 与 ${arch} 选 natives jar 解压到版本目录供 java.library.path，非空目录直接复用

use crate::errors::LauncherError;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn extract_natives(
    version_json: &serde_json::Value,
    version_dir: &Path,
    libraries_base_dir: &Path,
    current_os: &str,
    emit: &impl Fn(&str, String),
) -> Result<PathBuf, LauncherError> {
    let natives_dir = version_dir.join("natives");
    emit(
        "log-debug",
        format!("Natives目录: {}", natives_dir.display()),
    );

    // 同一实例多开时只能有一个进程在解压；否则第二个进程会读到写了一半的文件
    let _natives_lock = crate::utils::file_lock::acquire(
        &version_dir.join(".natives.lock"),
        std::time::Duration::from_secs(30),
    )?;

    if natives_dir.exists() {
        if fs::read_dir(&natives_dir)
            .map(|mut d| d.next().is_some())
            .unwrap_or(false)
        {
            emit(
                "log-debug",
                "Natives目录已存在且非空，跳过解压（使用缓存）".to_string(),
            );
            return Ok(natives_dir);
        }
        fs::remove_dir_all(&natives_dir)?;
    }
    fs::create_dir_all(&natives_dir)?;

    let Some(libraries) = version_json["libraries"].as_array() else {
        return Ok(natives_dir);
    };

    for lib in libraries {
        let Some(natives) = lib.get("natives") else {
            continue;
        };

        emit("log-debug", format!("发现Natives库: {:?}", lib));

        let Some(os_classifier) = natives.get(current_os).and_then(|v| v.as_str()) else {
            continue;
        };

        // 分类符里的 ${arch} 需替换成运行架构
        let arch = if std::env::consts::ARCH.contains("64") {
            "64"
        } else {
            "32"
        };
        let classifier = os_classifier.replace("${arch}", arch);

        emit(
            "log-debug",
            format!(
                "正在查找的OS分类器: {} (原始: {})",
                classifier, os_classifier
            ),
        );

        let Some(artifact) = lib
            .get("downloads")
            .and_then(|d| d.get("classifiers"))
            .and_then(|c| c.get(&classifier))
        else {
            continue;
        };

        emit("log-debug", format!("Natives Artifact: {:?}", artifact));

        let lib_path = libraries_base_dir.join(artifact["path"].as_str().unwrap_or(""));
        emit(
            "log-debug",
            format!("尝试解压Natives库: {}", lib_path.display()),
        );

        if !lib_path.exists() {
            emit(
                "log-error",
                format!("Natives库文件不存在: {}", lib_path.display()),
            );
            return Err(LauncherError::Custom(format!(
                "Natives库文件不存在: {}",
                lib_path.display()
            )));
        }

        extract_native_jar(&lib_path, &natives_dir, lib, emit)?;
        log_natives_dir_contents(&natives_dir, emit);
    }

    Ok(natives_dir)
}

fn extract_native_jar(
    lib_path: &Path,
    natives_dir: &Path,
    lib: &serde_json::Value,
    emit: &impl Fn(&str, String),
) -> Result<(), LauncherError> {
    let file = fs::File::open(lib_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let entry_name = file.name()?.to_string();

        if should_exclude_entry(&entry_name, lib) {
            continue;
        }

        if entry_name.ends_with('/') {
            continue;
        }

        // 取出最后一段文件名，避免嵌套目录
        let file_stem = Path::new(&entry_name)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&entry_name);

        let outpath = natives_dir.join(file_stem);

        if let Some(p) = outpath.parent()
            && !p.exists()
        {
            fs::create_dir_all(p)?;
        }

        let mut outfile = fs::File::create(&outpath)?;
        io::copy(&mut file, &mut outfile)?;

        emit(
            "log-debug",
            format!("解压Natives文件: {} -> {}", entry_name, outpath.display()),
        );
    }

    Ok(())
}

fn should_exclude_entry(entry_name: &str, lib: &serde_json::Value) -> bool {
    let Some(extract_rules) = lib.get("extract") else {
        return false;
    };

    let Some(exclude) = extract_rules.get("exclude").and_then(|e| e.as_array()) else {
        return false;
    };

    exclude
        .iter()
        .any(|v| entry_name.starts_with(v.as_str().unwrap_or("")))
}

fn log_natives_dir_contents(natives_dir: &Path, emit: &impl Fn(&str, String)) {
    if !natives_dir.exists() {
        return;
    }

    if let Ok(entries) = fs::read_dir(natives_dir) {
        let names: Vec<String> = entries
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();

        emit(
            "log-debug",
            format!("natives 目录内容: [{}]", names.join(", ")),
        );
    }
}
