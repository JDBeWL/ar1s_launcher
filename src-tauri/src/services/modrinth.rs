//! Modrinth API 模块
//!
//! 封装 api.modrinth.com/v2 的搜索与项目接口，请求失败按指数退避重试 3 次

use crate::errors::LauncherError;
use crate::models::DownloadJob;
use crate::models::modpack::*;
use crate::services::download;
use crate::services::http_client;
use log::{debug, warn};
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

const MODRINTH_API_BASE: &str = "https://api.modrinth.com/v2";
const MAX_API_RETRIES: u32 = 3;

pub struct ModrinthService {
    client: &'static Client,
}

impl ModrinthService {
    pub fn new() -> Self {
        Self {
            client: http_client::get_client(),
        }
    }

    async fn retry_request(
        &self,
        request_fn: impl Fn(&Client) -> reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, LauncherError> {
        let mut last_err = None;
        for attempt in 0..=MAX_API_RETRIES {
            let builder = request_fn(self.client);
            match builder.send().await {
                Ok(resp) => return Ok(resp),
                Err(e) => {
                    if attempt < MAX_API_RETRIES {
                        let backoff = std::time::Duration::from_millis(500 * (1 << attempt));
                        warn!(
                            "Modrinth API 请求失败 (尝试 {}/{}): {}, {}ms 后重试",
                            attempt + 1,
                            MAX_API_RETRIES + 1,
                            e,
                            backoff.as_millis()
                        );
                        tokio::time::sleep(backoff).await;
                    }
                    last_err = Some(e);
                }
            }
        }
        Err(LauncherError::Custom(format!(
            "Modrinth API 请求失败 (已重试 {} 次): {}",
            MAX_API_RETRIES,
            last_err.unwrap()
        )))
    }

    pub async fn search_modpacks(
        &self,
        query: Option<String>,
        game_versions: Option<Vec<String>>,
        loaders: Option<Vec<String>>,
        categories: Option<Vec<String>>,
        limit: Option<u32>,
        offset: Option<u32>,
        sort_by: Option<String>,
    ) -> Result<ModrinthSearchResponse, LauncherError> {
        let mut params = HashMap::new();

        let search_query = query.unwrap_or_else(|| "*".to_string());
        params.insert("query", search_query);

        if let Some(sort_val) = sort_by {
            params.insert("index", sort_val);
        }

        // facets 是分组数组：外层 AND、组内 OR
        let mut facets_groups: Vec<Vec<String>> = vec![vec!["project_type:modpack".to_string()]];

        if let Some(versions) = game_versions {
            // 同组放多个版本才是 OR；这里每个版本单独一组（AND，更严格）
            for v in &versions {
                facets_groups.push(vec![format!("versions:{}", v)]);
            }
            debug!("添加游戏版本过滤: {:?}", versions);
        }

        if let Some(loader_list) = loaders {
            for loader in &loader_list {
                facets_groups.push(vec![format!("categories:{}", loader)]);
            }
            debug!("添加加载器过滤: {:?}", loader_list);
        }

        if let Some(category_list) = categories {
            for category in &category_list {
                facets_groups.push(vec![format!("categories:{}", category)]);
            }
            debug!("添加分类过滤: {:?}", category_list);
        }

        let facets_json = serde_json::to_string(&facets_groups)?;
        params.insert("facets", facets_json.clone());
        debug!("生成的 facets 参数: {}", facets_json);

        params.insert("limit", limit.unwrap_or(20).to_string());
        params.insert("offset", offset.unwrap_or(0).to_string());

        let url = format!("{}/search", MODRINTH_API_BASE);
        let response = self
            .retry_request(|client| client.get(&url).query(&params))
            .await
            .map_err(|e| LauncherError::Custom(format!("搜索整合包失败: {}", e)))?;

        if !response.status().is_success() {
            return Err(LauncherError::Custom(format!(
                "Modrinth API返回错误: {}",
                response.status()
            )));
        }

        let json_response: Value = response
            .json()
            .await
            .map_err(|e| LauncherError::Custom(format!("解析响应失败: {}", e)))?;

        let hits = json_response["hits"]
            .as_array()
            .ok_or_else(|| LauncherError::Custom("无效的响应格式".to_string()))?
            .iter()
            .filter_map(|hit| {
                // 搜索结果可能混入模组，只保留 modpack 项目
                match hit.get("project_type").and_then(|v| v.as_str()) {
                    Some("modpack") => {}
                    _ => return None,
                }
                // 搜索结果字段名不同于项目接口：游戏版本在 `versions`，无独立 loaders（混在 `categories`），分类字段是 `categories`
                Some(ModrinthModpack {
                    slug: hit["slug"].as_str()?.to_string(),
                    title: hit["title"].as_str()?.to_string(),
                    description: hit["description"].as_str().unwrap_or("").to_string(),
                    icon_url: hit["icon_url"].as_str().map(|s| s.to_string()),
                    author: hit["author"].as_str()?.to_string(),
                    downloads: hit["downloads"].as_u64().unwrap_or(0),
                    date_created: hit["date_created"].as_str()?.to_string(),
                    date_modified: hit["date_modified"].as_str()?.to_string(),
                    latest_version: hit["latest_version"].as_str()?.to_string(),
                    game_versions: hit["versions"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default(),
                    loaders: hit["categories"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| {
                                    let cat = v.as_str()?;
                                    match cat {
                                        "fabric" | "forge" | "quilt" | "neoforge" => {
                                            Some(cat.to_string())
                                        }
                                        _ => None,
                                    }
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                    categories: hit["categories"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| {
                                    let cat = v.as_str()?;
                                    match cat {
                                        "fabric" | "forge" | "quilt" | "neoforge" => None,
                                        _ => Some(cat.to_string()),
                                    }
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                })
            })
            .collect();

        let total_hits = json_response["total_hits"].as_u64().unwrap_or(0) as u32;

        Ok(ModrinthSearchResponse { hits, total_hits })
    }

    pub async fn get_modpack(&self, slug_or_id: &str) -> Result<ModrinthModpack, LauncherError> {
        let url = format!("{}/project/{}", MODRINTH_API_BASE, slug_or_id);
        let response = self
            .retry_request(|client| client.get(&url))
            .await
            .map_err(|e| LauncherError::Custom(format!("获取整合包信息失败: {}", e)))?;

        if !response.status().is_success() {
            return Err(LauncherError::Custom(format!(
                "获取整合包信息失败: {}",
                response.status()
            )));
        }

        let project: Value = response
            .json()
            .await
            .map_err(|e| LauncherError::Custom(format!("解析响应失败: {}", e)))?;

        Ok(ModrinthModpack {
            slug: project["slug"]
                .as_str()
                .ok_or_else(|| LauncherError::Custom("缺少slug字段".to_string()))?
                .to_string(),
            title: project["title"]
                .as_str()
                .ok_or_else(|| LauncherError::Custom("缺少title字段".to_string()))?
                .to_string(),
            description: project["description"].as_str().unwrap_or("").to_string(),
            icon_url: project["icon_url"].as_str().map(|s| s.to_string()),
            author: project["author"]
                .as_str()
                .or_else(|| project["team"].as_str())
                .unwrap_or("Unknown")
                .to_string(),
            downloads: project["downloads"].as_u64().unwrap_or(0),
            date_created: project["date_created"].as_str().unwrap_or("").to_string(),
            date_modified: project["date_modified"].as_str().unwrap_or("").to_string(),
            // 项目接口没有 latest_version 字段，取 versions（版本 ID 数组）的第一个
            latest_version: project["versions"]
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            game_versions: project["game_versions"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            loaders: project["loaders"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            categories: project["categories"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
        })
    }

    pub async fn get_modpack_versions(
        &self,
        project_id: &str,
        game_versions: Option<Vec<String>>,
        loaders: Option<Vec<String>>,
    ) -> Result<Vec<ModrinthModpackVersion>, LauncherError> {
        let mut params = HashMap::new();

        if let Some(versions) = game_versions {
            params.insert("game_versions", serde_json::to_string(&versions)?);
        }

        if let Some(loader_list) = loaders {
            params.insert("loaders", serde_json::to_string(&loader_list)?);
        }

        let url = format!("{}/project/{}/version", MODRINTH_API_BASE, project_id);
        let response = self
            .retry_request(|client| client.get(&url).query(&params))
            .await
            .map_err(|e| LauncherError::Custom(format!("获取整合包版本失败: {}", e)))?;

        if !response.status().is_success() {
            return Err(LauncherError::Custom(format!(
                "获取整合包版本失败: {}",
                response.status()
            )));
        }

        let versions: Vec<Value> = response
            .json()
            .await
            .map_err(|e| LauncherError::Custom(format!("解析响应失败: {}", e)))?;

        versions
            .into_iter()
            .map(|version| {
                Ok(ModrinthModpackVersion {
                    id: version["id"]
                        .as_str()
                        .ok_or_else(|| LauncherError::Custom("缺少id字段".to_string()))?
                        .to_string(),
                    name: version["name"]
                        .as_str()
                        .ok_or_else(|| LauncherError::Custom("缺少name字段".to_string()))?
                        .to_string(),
                    version_number: version["version_number"]
                        .as_str()
                        .ok_or_else(|| LauncherError::Custom("缺少version_number字段".to_string()))?
                        .to_string(),
                    game_versions: version["game_versions"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default(),
                    loaders: version["loaders"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default(),
                    featured: version["featured"].as_bool().unwrap_or(false),
                    date_published: version["date_published"]
                        .as_str()
                        .ok_or_else(|| LauncherError::Custom("缺少date_published字段".to_string()))?
                        .to_string(),
                    downloads: version["downloads"].as_u64().unwrap_or(0),
                    files: version["files"]
                        .as_array()
                        .map(|files| {
                            files
                                .iter()
                                .filter_map(|file| {
                                    Some(ModrinthFile {
                                        url: file["url"].as_str()?.to_string(),
                                        filename: file["filename"].as_str()?.to_string(),
                                        primary: file["primary"].as_bool().unwrap_or(false),
                                        size: file["size"].as_u64().unwrap_or(0),
                                        hashes: ModrinthHashes {
                                            sha1: file["hashes"]["sha1"].as_str()?.to_string(),
                                            sha512: file["hashes"]["sha512"].as_str()?.to_string(),
                                        },
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                    dependencies: version["dependencies"]
                        .as_array()
                        .map(|deps| {
                            deps.iter()
                                .filter_map(|dep| {
                                    Some(ModrinthDependency {
                                        version_id: dep["version_id"]
                                            .as_str()
                                            .map(|s| s.to_string()),
                                        project_id: dep["project_id"]
                                            .as_str()
                                            .map(|s| s.to_string()),
                                        dependency_type: dep["dependency_type"]
                                            .as_str()?
                                            .to_string(),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                })
            })
            .collect()
    }

    pub async fn download_modpack_file(
        &self,
        file_url: &str,
        destination: &std::path::Path,
        size: u64,
        sha1: Option<&str>,
        global_cancel: &Arc<AtomicBool>,
    ) -> Result<(), LauncherError> {
        if global_cancel.load(Ordering::Acquire) {
            return Err(LauncherError::Cancelled);
        }

        let job = DownloadJob {
            url: file_url.to_string(),
            fallback_url: None,
            path: destination.to_path_buf(),
            size,
            hash: sha1
                .filter(|hash| !hash.trim().is_empty())
                .map(str::to_string),
        };
        let state = Arc::new(AtomicBool::new(true));
        let bytes_downloaded = Arc::new(AtomicU64::new(0));
        let bytes_since_last = Arc::new(AtomicU64::new(0));

        let mut last_error = None;
        for attempt in 0..3 {
            if global_cancel.load(Ordering::Acquire) {
                return Err(LauncherError::Cancelled);
            }

            match download::download_file(
                download::get_http_client(),
                &job,
                file_url,
                &state,
                global_cancel,
                &bytes_downloaded,
                &bytes_since_last,
            )
            .await
            {
                Ok(()) => return Ok(()),
                Err(e) => {
                    if e.is_cancelled() {
                        return Err(e);
                    }
                    warn!("整合包文件下载失败 (第 {} 次): {}", attempt + 1, e);
                    last_error = Some(e);
                    if attempt < 2 {
                        tokio::time::sleep(std::time::Duration::from_secs(1 << attempt)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| LauncherError::Custom("整合包文件下载失败".to_string())))
    }
}
