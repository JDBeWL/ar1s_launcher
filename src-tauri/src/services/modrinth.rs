use crate::errors::LauncherError;
use crate::models::modpack::*;
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;

const MODRINTH_API_BASE: &str = "https://api.modrinth.com/v2";
const USER_AGENT: &str = "Ar1sLauncher/1.0.0 (https://github.com/your-username/ar1s-launcher)";

pub struct ModrinthService {
    client: Client,
}

impl ModrinthService {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// 搜索整合包
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
        
        // 如果没有查询参数，使用默认查询来获取热门整合包
        let search_query = query.unwrap_or_else(|| "*".to_string());
        params.insert("query", search_query);

        if let Some(sort_val) = sort_by {
            params.insert("index", sort_val);
        }
        
        // 正确构建 facets：每个条件一个分组（分组之间 AND，同组内 OR）
        let mut facets_groups: Vec<Vec<String>> = vec![vec!["project_type:modpack".to_string()]];

        if let Some(versions) = game_versions {
            // 要求命中任意一个所选版本时，可将多个版本放入同一组；
            // 这里我们只有单选，直接每个版本一个 AND 组更严格
            for v in &versions {
                facets_groups.push(vec![format!("versions:{}", v)]);
            }
            println!("添加游戏版本过滤: {:?}", versions);
        }

        if let Some(loader_list) = loaders {
            // 加载器作为 AND 条件
            for loader in &loader_list {
                facets_groups.push(vec![format!("categories:{}", loader)]);
            }
            println!("添加加载器过滤: {:?}", loader_list);
        }

        if let Some(category_list) = categories {
            // 分类作为 AND 条件
            for category in &category_list {
                facets_groups.push(vec![format!("categories:{}", category)]);
            }
            println!("添加分类过滤: {:?}", category_list);
        }

        // 写入 facets 参数
        let facets_json = serde_json::to_string(&facets_groups)?;
        params.insert("facets", facets_json.clone());
        println!("生成的 facets 参数: {}", facets_json);
        
        params.insert("limit", limit.unwrap_or(20).to_string());
        params.insert("offset", offset.unwrap_or(0).to_string());
        
        let url = format!("{}/search", MODRINTH_API_BASE);
        let response = self
            .client
            .get(&url)
            .header("User-Agent", USER_AGENT)
            .query(&params)
            .send()
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
        
        // 转换API响应到我们的数据结构
        let hits = json_response["hits"]
            .as_array()
            .ok_or_else(|| LauncherError::Custom("无效的响应格式".to_string()))?
            .iter()
            .filter_map(|hit| {
                // 仅保留整合包项目，防止误返回模组结果
                match hit.get("project_type").and_then(|v| v.as_str()) {
                    Some("modpack") => {}
                    _ => return None,
                }
                // 根据实际API响应结构解析数据
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
                    // 注意：实际API中版本字段是 "versions"，不是 "game_versions"
                    game_versions: hit["versions"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default(),
                    // 注意：实际API中没有 "loaders" 字段，需要从 categories 中提取
                    loaders: hit["categories"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| {
                                    let cat = v.as_str()?;
                                    // 只保留加载器相关的分类
                                    match cat {
                                        "fabric" | "forge" | "quilt" | "neoforge" => Some(cat.to_string()),
                                        _ => None,
                                    }
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                    // 注意：实际API中分类字段是 "categories"
                    categories: hit["categories"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| {
                                    let cat = v.as_str()?;
                                    // 排除加载器分类，只保留其他分类
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
        
        let total_hits = json_response["total_hits"]
            .as_u64()
            .unwrap_or(0) as u32;
        
        Ok(ModrinthSearchResponse { hits, total_hits })
    }

    /// 获取整合包详细信息
    pub async fn get_modpack(&self, slug_or_id: &str) -> Result<ModrinthModpack, LauncherError> {
        let url = format!("{}/project/{}", MODRINTH_API_BASE, slug_or_id);
        let response = self
            .client
            .get(&url)
            .header("User-Agent", USER_AGENT)
            .send()
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
            slug: project["slug"].as_str().ok_or_else(|| LauncherError::Custom("缺少slug字段".to_string()))?.to_string(),
            title: project["title"].as_str().ok_or_else(|| LauncherError::Custom("缺少title字段".to_string()))?.to_string(),
            description: project["description"].as_str().unwrap_or("").to_string(),
            icon_url: project["icon_url"].as_str().map(|s| s.to_string()),
            author: project["author"].as_str().ok_or_else(|| LauncherError::Custom("缺少author字段".to_string()))?.to_string(),
            downloads: project["downloads"].as_u64().unwrap_or(0),
            date_created: project["date_created"].as_str().ok_or_else(|| LauncherError::Custom("缺少date_created字段".to_string()))?.to_string(),
            date_modified: project["date_modified"].as_str().ok_or_else(|| LauncherError::Custom("缺少date_modified字段".to_string()))?.to_string(),
            latest_version: project["latest_version"].as_str().ok_or_else(|| LauncherError::Custom("缺少latest_version字段".to_string()))?.to_string(),
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

    /// 获取整合包版本列表
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
            .client
            .get(&url)
            .header("User-Agent", USER_AGENT)
            .query(&params)
            .send()
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
                    id: version["id"].as_str().ok_or_else(|| LauncherError::Custom("缺少id字段".to_string()))?.to_string(),
                    name: version["name"].as_str().ok_or_else(|| LauncherError::Custom("缺少name字段".to_string()))?.to_string(),
                    version_number: version["version_number"].as_str().ok_or_else(|| LauncherError::Custom("缺少version_number字段".to_string()))?.to_string(),
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
                    date_published: version["date_published"].as_str().ok_or_else(|| LauncherError::Custom("缺少date_published字段".to_string()))?.to_string(),
                    downloads: version["downloads"].as_u64().unwrap_or(0),
                    files: version["files"]
                        .as_array()
                        .map(|files| {
                            files.iter().filter_map(|file| {
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
                            }).collect()
                        })
                        .unwrap_or_default(),
                    dependencies: version["dependencies"]
                        .as_array()
                        .map(|deps| {
                            deps.iter().filter_map(|dep| {
                                Some(ModrinthDependency {
                                    version_id: dep["version_id"].as_str().map(|s| s.to_string()),
                                    project_id: dep["project_id"].as_str().map(|s| s.to_string()),
                                    dependency_type: dep["dependency_type"].as_str()?.to_string(),
                                })
                            }).collect()
                        })
                        .unwrap_or_default(),
                })
            })
            .collect()
    }

    /// 下载整合包文件
    pub async fn download_modpack_file(
        &self,
        file_url: &str,
        destination: &std::path::Path,
    ) -> Result<(), LauncherError> {
        let response = self
            .client
            .get(file_url)
            .header("User-Agent", USER_AGENT)
            .send()
            .await
            .map_err(|e| LauncherError::Custom(format!("下载文件失败: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(LauncherError::Custom(format!(
                "下载文件失败: {}",
                response.status()
            )));
        }
        
        let content = response
            .bytes()
            .await
            .map_err(|e| LauncherError::Custom(format!("读取文件内容失败: {}", e)))?;
        
        tokio::fs::write(destination, content)
            .await
            .map_err(|e| LauncherError::Custom(format!("保存文件失败: {}", e)))?;
        
        Ok(())
    }
}