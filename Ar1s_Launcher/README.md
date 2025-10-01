# Ar1s Launcher

一个使用 Tauri v2 + Vue 3 构建的跨平台 Minecraft 启动器，集成版本下载、文件校验、Java 管理、版本隔离、内存配置和一键启动等能力。

## 技术栈

- 前端
  - Vue 3 + Vite 6 + TypeScript
  - Vuetify 3 (Material Design 3)
  - Pinia（状态管理）
  - Vue Router
  - @mdi/font 图标
- 桌面端
  - Tauri 2.0 (beta)
  - Rust（核心逻辑：下载/校验/启动/配置/Java）
  - 主要 crate：reqwest, tokio, serde, zip, sysinfo, uuid, md-5/sha1, tauri-plugins (dialog/http/fs/opener)

## 功能概览

- 版本下载：支持官方源与 BMCL 镜像，断点重试、整体进度广播、完成/错误/取消状态回传
- 文件校验：启动前对主 JAR、库文件、natives 等进行存在性/完整性检查
- 启动游戏：自动构建 classpath、解压 natives、处理新版/旧版参数（arguments/minecraftArguments），支持版本隔离目录
- Java 管理：自动扫描本机 Java、校验可用性、保存路径（支持 PATH 中 java）
- 配置持久化：ar1s.json（与可执行文件同目录），含 game_dir、max_memory、download_threads、version_isolation、username/uuid 等
- UI/UX：MD3 风格主题、深浅色切换、下载全局通知、侧边导航、设置页滑条与输入同步

## 安装与运行

前置条件
- Node.js 18+ 与包管理器（推荐 npm 或 pnpm）
- Rust 稳定工具链（包含 cargo）
- Tauri 依赖（根据平台安装：Windows 需 VS Build Tools、WebView2；macOS 需 Xcode CLT；Linux 需 GTK/openssl 等）
- 参考官方文档：https://tauri.app/ 或 VS Code Tauri 插件指引

安装依赖
- npm i

仅运行前端（Vite）
- npm run dev
- 打开 http://localhost:1420（仅用于前端开发预览；完整功能请使用 tauri dev）

以 Tauri 形式开发调试
- npm run tauri dev
- 前端服务由 Tauri 按 tauri.conf.json 启动，Rust 命令可用

构建前端产物
- npm run build

打包生成桌面应用
- npm run tauri build
- 产物位置由 Tauri 打包系统生成（各平台安装包/可执行文件）

## 使用指南

- 下载
  - 进入“下载”页面，选择源（官方/BMCL），检索/排序版本，点击“下载”
  - 右上角刷新按钮可重新获取版本清单
  - 下载进度会以通知卡形式显示，可取消下载或隐藏通知（可用 FAB 恢复）
- 启动
  - 在“启动”页面选择已安装的版本，系统会自动验证缺失文件
  - 配置用户名、离线模式，点击“启动 Minecraft”
- 设置
  - 选择游戏目录，是否启用版本隔离
  - 配置最大内存（滑条与输入框同步），下载线程数
  - 管理 Java 路径（自动搜索、校验），也可使用 PATH 中的 java
- 主题
  - 标题栏右侧可切换暗/亮主题，结果持久于 localStorage

## 配置与数据位置

- 配置文件：ar1s.json（与可执行文件同目录）
- 默认游戏目录：可执行文件同目录下 .minecraft（首次运行会创建：versions、libraries、assets、saves、resourcepacks、logs）
- 日志：
  - 游戏目录 logs/version_fetch.log（版本清单过程）
  - 当前工作目录 logs/network_debug.log（网络调试）

## 常见问题

- 无法找到 Java
  - 在设置页使用“自动查找”或手动选择 java.exe；或确保系统 PATH 可直接运行 java -version
- 下载失败/超时
  - 切换下载源至 BMCL；检查网络；重试会指数退避；部分资源支持 fallback URL
- 启动失败
  - 在“启动”页查看缺失文件提示；先到“下载”页补全；确认 Java 路径与内存设置合理
- 窗口无边框
  - 已提供自定义最小化/最大化/关闭按钮（App.vue 顶栏右侧）

## 备注

- 版本参数兼容新版 arguments 与旧版 minecraftArguments
- natives 解压按库 extract.exclude 规则过滤
- 下载校验使用 SHA1（与官方元数据一致），旧格式无哈希时跳过校验
- Windows 默认隐藏游戏子进程控制台窗口