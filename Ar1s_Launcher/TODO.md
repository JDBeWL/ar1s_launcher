# Ar1s Launcher

优先级说明：
- P0：高优先，近期必须完成
- P1：中优先，影响质量或体验
- P2：低优先，优化/美化

## 配置与版本
- [x] P0 迁移 Tauri 后端及插件到 2.x 稳定版：`tauri`、`tauri-plugin-*`
- [x] P0 在 `tauri.conf.json` 配置合理 CSP（非 null），限制默认脚本来源、禁用不必要的远程脚本
- [ ] P1 检查 `bundle.targets` 是否需要 "all"，如仅发布 Windows 可改为 ["windows"] 以加快打包
- [ ] P1 在 `README.md` 补充开发/构建/打包步骤与常见问题；持续维护本 `TODO.md`
- [ ] P2 统一图标资源与品牌细节，校验各平台图标尺寸与路径完整性

## Rust 后端（src-tauri）
### downloads.rs
- [ ] P0 将原子状态（AtomicU64）重构为结构体 `DownloadState`（files_downloaded、bytes_downloaded、total_files、total_bytes、last_update_ts、last_error 等），通过 `Mutex/RwLock` 或 `tokio::sync` 共享（满足现有 TODO）
- [ ] P0 消除 `unwrap/expect`（如 `semaphore.acquire_owned().await.unwrap`），统一返回 `Result` 并在调用处处理
- [ ] P0 替换 `println!/eprintln!` 为 `log` 宏（`info!/debug!/warn!/error!`）；使用 `fern` 初始化日志输出到控制台与 `src-tauri/logs`
- [ ] P0 为断点续传与重试逻辑设定上限，达到 `MAX_JOB_RETRIES` 后上报明确错误事件
- [ ] P1 将 `serde_json::Value` 动态解析替换为静态结构体映射，减少 `as_str().unwrap` 风险
- [ ] P1 统一校验算法（如 SHA1）并与服务端索引保持一致；为 `verify_file` 增加容错与错误信息
- [ ] P1 进度事件节流：按固定时间/字节阈值批量上报，降低前端渲染压力

### launcher.rs
- [ ] P0 移除 DEBUG/ERROR 的 `println!`，统一使用日志与事件（已有 `window.emit`）
- [ ] P1 文件存在性与路径校验返回自定义错误（`thiserror`），避免字符串拼接错误
- [ ] P1 使用结构体解析 Natives 解压规则，替代多层 `.get().and_then().unwrap`
- [ ] P1 统一 Classpath 构建与路径分隔符；对缺失库给出修复策略或自动恢复

### 其他
- [ ] P1 在 `main.rs` 增加日志初始化（fern）与全局错误钩子；`setup` 阶段改用 `info!`
- [ ] P1 添加关键服务的单元测试与集成测试（下载校验、重试、事件上报），使用临时目录与 `#[cfg(test)]`

## 前端（Vue + Pinia）
### 类型与事件
- [ ] P0 为 `download-progress`/`launch-command`/`game-exit` 等事件定义 TypeScript 接口；移除 `any`（`vite-env.d.ts`、`stores/downloadStore.ts`）
- [ ] P0 封装事件订阅/反订阅为 composable（如 `useTauriEvents`）或统一在 store 管理，组件卸载时清理监听器
- [ ] P1 移除临时 `console.log`；通过环境变量控制日志级别，生产环境禁用调试输出
- [ ] P1 在 Pinia 中定义 `DownloadProgress` 完整结构（status、bytesDownloaded、filesDownloaded、totalBytes、totalFiles、message、attempt 等）并与后端保持一致

### UI/UX
- [ ] P1 统一进度条与通知组件（`GlobalDownloadStatus.vue`、`NotificationDownload.vue`），支持错误恢复/重试
- [ ] P1 在 `SettingsView.vue` 增加 Java 路径选择与校验，调用后端 `java_controller`
- [ ] P2 自定义无边框窗口标题栏与拖拽区域（`decorations:false`），完善最小化/关闭按钮样式

## 日志与可观测性
- [ ] P0 后端使用 `fern` 初始化文件+控制台双输出，日志按日期滚动保存到 `src-tauri/logs`
- [ ] P1 前端错误统一通过对话框/吐司提示，并可选上报到后端或写入本地日志
- [ ] P1 为关键路径添加结构化事件 payload（JSON），便于调试与追踪

## 安全与健壮性
- [ ] P0 审核并最小化 `opener`/`dialog`/`fs`/`http` 插件权限与使用范围
- [ ] P0 前后端输入校验与路径安全（避免路径遍历）；所有文件操作限定在安全目录下
- [ ] P1 统一错误码与用户可读消息，避免泄露内部路径与实现细节

## 测试与质量保障
- [ ] P1 前端：引入 Vitest + Vue Test Utils，测试事件渲染与状态更新
- [ ] P1 集成测试：模拟下载事件序列与失败重试，验证 UI 与状态一致性
- [ ] P2 Lint/格式化：启用 ESLint + Prettier 与 `rustfmt`/`clippy`，在 CI 步骤执行

## 版本与发布
- [ ] P1 在 `package.json` 增加快捷脚本：`tauri:dev`、`tauri:build`（可按平台区分）
- [ ] P1 文档化发布流程（签名、图标、版本号同步），保证 `tauri.conf.json` 与 `package.json` 版本一致

---
