<template>
  <v-container>
    <v-card>
      <v-card-title class="d-flex mt-2 align-center"> 添加新实例 </v-card-title>
      <v-card-text>
        <!-- 安装方式选择 -->
        <v-row class="mb-4">
          <v-col cols="12" class="d-flex align-center">
            <div class="d-flex align-center" style="width: 100%">
              <div
                class="install-type-tab flex-grow-1 text-center py-3 cursor-pointer"
                :class="{ 'install-type-active': installType === 'custom' }"
                @click="installType = 'custom'"
              >
                自定义安装
              </div>
              <div class="install-type-divider"></div>
              <div
                class="install-type-tab flex-grow-1 text-center py-3 cursor-pointer"
                :class="{ 'install-type-active': installType === 'online' }"
                @click="installType = 'online'"
              >
                从互联网安装
              </div>
            </div>
          </v-col>
        </v-row>

        <!-- 自定义安装内容 -->
        <div v-if="installType === 'custom'">
          <v-row>
            <v-col cols="12">
              <v-text-field
                v-model="instanceName"
                label="实例名称"
                :placeholder="defaultInstanceName"
                hide-details
              ></v-text-field>
            </v-col>
          </v-row>

          <!-- 搜索游戏版本 -->
          <v-row no-gutters class="align-center mt-4 mb-4">
            <v-col class="flex-grow-1 pr-2">
              <v-text-field
                v-model="searchVersion"
                label="搜索版本"
                prepend-inner-icon="mdi-magnify"
                clearable
                hide-details
              ></v-text-field>
            </v-col>
            <v-col class="shrink pr-2" style="max-width: 150px">
              <v-select
                v-model="versionTypeFilter"
                label="版本类型"
                :items="versionTypes"
                hide-details
              ></v-select>
            </v-col>
            <v-col class="shrink" style="max-width: 180px">
              <v-select
                v-model="sortOrder"
                label="排序方式"
                :items="sortOptions"
                hide-details
              ></v-select>
            </v-col>
          </v-row>

          <!-- 游戏版本和Mod加载器选择 -->
          <v-row no-gutters class="align-center mb-4">
            <v-col class="shrink pr-2" style="max-width: 200px">
              <v-select
                v-model="selectedVersion"
                :items="filteredVersions"
                item-title="id"
                item-value="id"
                label="游戏版本"
                :loading="loadingVersions"
                hide-details
                return-object
              ></v-select>
            </v-col>
            <v-col class="shrink pr-2" style="max-width: 200px">
              <v-select
                v-model="selectedModLoaderType"
                :items="modLoaderTypes"
                label="Mod加载器"
                :disabled="!selectedVersion"
                hide-details
              ></v-select>
            </v-col>
            <v-col class="shrink" style="max-width: 1000px">
              <v-select
                v-model="selectedModLoaderVersion"
                :items="modLoaderVersions"
                item-title="version"
                item-value="version"
                label="Mod加载器版本"
                :loading="loadingModLoaderVersions"
                :disabled="
                  !selectedModLoaderType || selectedModLoaderType === 'None'
                "
                placeholder="请先选择Mod加载器"
                hide-details
                return-object
              ></v-select>
            </v-col>
          </v-row>

          <!-- 进度条 -->
          <v-row v-if="showProgress" class="mt-4">
            <v-col cols="12">
              <div class="d-flex align-center justify-space-between mb-2">
                <span class="text-caption">{{ progressText }}</span>
                <span class="text-caption font-weight-medium"
                  >{{ progressValue }}%</span
                >
              </div>
              <v-progress-linear
                v-model="progressValue"
                color="primary"
                height="8"
                :indeterminate="progressIndeterminate"
              ></v-progress-linear>
            </v-col>
          </v-row>

          <!-- 开始安装按钮 -->
          <v-row class="mt-4">
            <v-col cols="12" class="text-right">
              <v-btn
                color="primary"
                size="large"
                @click="createInstance"
                :disabled="!selectedVersion || installing"
                :loading="installing"
              >
                开始安装
              </v-btn>
            </v-col>
          </v-row>
        </div>

        <!-- 从互联网安装内容 -->
        <div v-if="installType === 'online'">
          <!-- 平台选择 -->
          <v-row class="mb-4">
            <v-col cols="12">
              <div class="d-flex">
                <v-btn
                  :color="selectedPlatform === 'modrinth' ? 'primary' : 'grey lighten-3'"
                  :class="selectedPlatform === 'modrinth' ? 'elevation-4' : 'elevation-1'"
                  height="60"
                  width="200"
                  @click="selectedPlatform = 'modrinth'"
                  class="platform-btn mr-4"
                >
                  <span class="text-h6 font-weight-bold">Modrinth</span>
                </v-btn>
                <v-btn
                  :color="selectedPlatform === 'curseforge' ? 'primary' : 'grey lighten-3'"
                  :class="selectedPlatform === 'curseforge' ? 'elevation-4' : 'elevation-1'"
                  height="60"
                  width="200"
                  @click="selectedPlatform = 'curseforge'"
                  class="platform-btn"
                  disabled
                >
                  <span class="text-h6 font-weight-bold">CurseForge</span>
                  <v-chip small color="orange" class="ml-2">开发中</v-chip>
                </v-btn>
              </div>
            </v-col>
          </v-row>

          <!-- Modrinth整合包搜索 -->
          <div v-if="selectedPlatform === 'modrinth'">
            <!-- 搜索框和游戏版本 -->
            <v-row no-gutters class="align-center mb-4">
              <v-col class="flex-grow-1 pr-2">
                <v-text-field
                  v-model="modpackSearchQuery"
                  label="搜索整合包"
                  prepend-inner-icon="mdi-magnify"
                  clearable
                  hide-details
                  @input="searchModpacks"
                ></v-text-field>
              </v-col>
              <v-col class="shrink pr-2" style="max-width: 200px">
                <v-select
                  v-model="selectedGameVersion"
                  :items="gameVersions"
                  label="游戏版本"
                  clearable
                  hide-details
                  @update:model-value="searchModpacks"
                ></v-select>
              </v-col>
              <v-col class="shrink" style="max-width: 200px">
                <v-select
                  v-model="selectedLoader"
                  :items="loaders"
                  label="加载器"
                  clearable
                  hide-details
                  @update:model-value="searchModpacks"
                ></v-select>
              </v-col>
            </v-row>

            <!-- 其他筛选条件 -->
            <v-row no-gutters class="align-center mb-4">
              <v-col class="flex-grow-1 pr-2" style="min-width: 150px">
                <v-select
                  v-model="selectedCategory"
                  :items="categories"
                  label="分类"
                  clearable
                  hide-details
                  @update:model-value="searchModpacks"
                ></v-select>
              </v-col>
              <v-col class="flex-grow-1 pr-2" style="min-width: 150px">
                <v-select
                  v-model="sortBy"
                  :items="modpackSortOptions"
                  label="排序"
                  hide-details
                  @update:model-value="searchModpacks"
                ></v-select>
              </v-col>

            </v-row>

            <!-- 加载状态 -->
            <v-row v-if="loadingModpacks" class="mb-4">
              <v-col cols="12" class="text-center">
                <v-progress-circular indeterminate color="primary"></v-progress-circular>
                <div class="mt-2">正在搜索整合包...</div>
              </v-col>
            </v-row>

            <!-- 整合包列表 -->
            <v-row v-else-if="modpacks.length > 0" class="mb-4">
              <v-col 
                v-for="modpack in modpacks" 
                :key="modpack.slug" 
                cols="12" 
                sm="6" 
                md="4"
              >
                <v-card 
                  class="modpack-card" 
                  elevation="2"
                  @click="selectModpack(modpack)"
                  :class="{ 'modpack-card-selected': selectedModpack?.slug === modpack.slug }"
                >
                  <v-img
                    v-if="modpack.icon_url"
                    :src="modpack.icon_url"
                    height="120"
                    cover
                    class="modpack-image"
                  ></v-img>
                  <div v-else class="modpack-image-placeholder">
                    <v-icon size="48" color="grey">mdi-package-variant</v-icon>
                  </div>
                  
                  <v-card-title class="text-h6 modpack-title">
                    {{ modpack.title }}
                  </v-card-title>
                  
                  <v-card-text class="modpack-info">
                    <div class="modpack-author">作者: {{ modpack.author }}</div>
                    <div class="modpack-downloads">下载量: {{ formatNumber(modpack.downloads) }}</div>
                    <div class="modpack-versions">
                      支持版本: {{ formatVersionRange(modpack.game_versions) }}
                    </div>
                    <div class="modpack-loaders">
                      加载器: {{ formatLoaders(modpack.loaders) }}
                    </div>
                    <div class="modpack-categories" v-if="modpack.categories.length > 0">
                      分类: {{ modpack.categories.slice(0, 3).join(', ') }}
                    </div>
                    <div class="modpack-updated">
                      更新: {{ formatDate(modpack.date_modified) }}
                    </div>
                    <div class="modpack-description">
                      {{ truncateText(modpack.description, 100) }}
                    </div>
                  </v-card-text>
                </v-card>
              </v-col>
            </v-row>

            <!-- 分页组件 -->
            <v-row v-if="modpacks.length > 0 && modpackTotalPages > 1" class="mb-4">
              <v-col cols="12" class="d-flex justify-center">
                <div class="d-flex align-center">
                  <v-pagination
                    v-model="modpackCurrentPage"
                    :length="modpackTotalPages"
                    :total-visible="7"
                    @update:model-value="onModpackPageChange"
                    class="mr-4"
                  ></v-pagination>
                  <div class="text-caption text-grey">
                    共 {{ modpackTotalHits }} 个整合包
                  </div>
                </div>
              </v-col>
            </v-row>

            <!-- 空状态 -->
            <v-row v-else-if="!loadingModpacks && modpacks.length === 0" class="mb-4">
              <v-col cols="12" class="text-center">
                <v-icon size="64" color="grey">mdi-package-variant</v-icon>
                <div class="mt-2 text-grey">
                  {{ modpackInitialLoad ? '正在加载热门整合包...' : (modpackSearchQuery || selectedGameVersion || selectedLoader || selectedCategory ? '未找到相关整合包' : '请输入关键词搜索整合包') }}
                </div>
              </v-col>
            </v-row>

            <!-- 版本选择 -->
            <v-row v-if="selectedModpack" no-gutters class="align-center mb-4">
              <v-col class="shrink pr-2" style="max-width: 200px">
                <v-select
                  v-model="selectedModpackVersion"
                  :items="modpackVersions"
                  item-title="name"
                  item-value="id"
                  label="选择整合包版本"
                  :loading="loadingModpackVersions"
                  hide-details
                ></v-select>
              </v-col>
            </v-row>

            <!-- 实例名称 -->
            <v-row v-if="selectedModpack" no-gutters class="align-center mb-4">
              <v-col class="flex-grow-1">
                <v-text-field
                  v-model="modpackInstanceName"
                  :label="`实例名称 (默认: ${selectedModpack.slug})`"
                  :placeholder="selectedModpack.slug"
                  hide-details
                ></v-text-field>
              </v-col>
            </v-row>

            <!-- 进度条 -->
            <v-row v-if="showModpackProgress" class="mt-4">
              <v-col cols="12">
                <div class="d-flex align-center justify-space-between mb-2">
                  <span class="text-caption">{{ modpackProgressText }}</span>
                  <span class="text-caption font-weight-medium">
                    {{ modpackProgressValue }}%
                  </span>
                </div>
                <v-progress-linear
                  v-model="modpackProgressValue"
                  color="primary"
                  height="8"
                  :indeterminate="modpackProgressIndeterminate"
                ></v-progress-linear>
              </v-col>
            </v-row>

            <!-- 安装按钮 -->
            <v-row v-if="selectedModpack" class="mt-4">
              <v-col cols="12" class="text-right">
                <v-btn
                  color="primary"
                  size="large"
                  @click="installModpack"
                  :disabled="!selectedModpackVersion || installingModpack"
                  :loading="installingModpack"
                >
                  安装整合包
                </v-btn>
              </v-col>
            </v-row>
          </div>

          <!-- CurseForge整合包搜索 (占位) -->
          <div v-if="selectedPlatform === 'curseforge'">
            <v-alert type="info" class="mb-4">
              CurseForge整合包支持正在开发中...
            </v-alert>
          </div>
        </div>
      </v-card-text>
    </v-card>
  </v-container>
</template>

<script setup lang="ts">
import { ref, onMounted, watch, computed, onUnmounted } from "vue";
import { useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const installType = ref("custom");
const router = useRouter();

interface MinecraftVersion {
  id: string;
  type: string;
  url: string;
  time: string;
  releaseTime: string;
}

interface ModrinthModpack {
  slug: string;
  title: string;
  author: string;
  downloads: number;
  game_versions: string[];
  loaders: string[];
  description: string;
  icon_url?: string;
  date_created: string;
  date_modified: string;
  latest_version: string;
  categories: string[];
}

interface ModrinthVersion {
  id: string;
  name: string;
  version_number: string;
  game_versions: string[];
  loaders: string[];
  featured: boolean;
  date_published: string;
  downloads: number;
  files: ModrinthFile[];
  dependencies: ModrinthDependency[];
}

interface ModrinthFile {
  url: string;
  filename: string;
  primary: boolean;
  size: number;
  hashes: ModrinthHashes;
}

interface ModrinthHashes {
  sha1: string;
  sha512: string;
}

interface ModrinthDependency {
  version_id?: string;
  project_id?: string;
  dependency_type: string;
}

interface ModrinthSearchResult {
  hits: ModrinthModpack[];
  total_hits: number;
}

const versions = ref<MinecraftVersion[]>([]);
const loadingVersions = ref(false);
const selectedVersion = ref<MinecraftVersion | null>(null);
const searchVersion = ref("");
const versionTypeFilter = ref("release");
const sortOrder = ref("newest");

const versionTypes = [
  { title: "正式版", value: "release" },
  { title: "快照版", value: "snapshot" },
  { title: "全部", value: "all" },
];

const sortOptions = [
  { title: "最新优先", value: "newest" },
  { title: "最旧优先", value: "oldest" },
  { title: "A-Z", value: "az" },
  { title: "Z-A", value: "za" },
];

const instanceName = ref("");
const installing = ref(false);
const showProgress = ref(false);
const progressValue = ref(0);
const progressIndeterminate = ref(false);
const progressText = ref("");

const modLoaderTypes = ["None", "Forge", "Fabric", "Quilt"];
const selectedModLoaderType = ref("None");

const modLoaderVersions = ref<string[]>([]);
const loadingModLoaderVersions = ref(false);
const selectedModLoaderVersion = ref<string | null>(null);

// Modrinth整合包相关状态
const selectedPlatform = ref("modrinth");
const modpackSearchQuery = ref("");
const selectedGameVersion = ref<string | null>(null);
const selectedLoader = ref<string | null>(null);
const selectedCategory = ref<string | null>(null);
const sortBy = ref("relevance");
const modpacks = ref<ModrinthModpack[]>([]);
const loadingModpacks = ref(false);
const selectedModpack = ref<ModrinthModpack | null>(null);
const modpackVersions = ref<ModrinthVersion[]>([]);
const loadingModpackVersions = ref(false);
const selectedModpackVersion = ref<string | null>(null);
const modpackInstanceName = ref("");
const installingModpack = ref(false);
const showModpackProgress = ref(false);
const modpackProgressValue = ref(0);
const modpackProgressIndeterminate = ref(false);
const modpackProgressText = ref("");

// 分页相关状态
const modpackCurrentPage = ref(1);
const modpackItemsPerPage = 12;
const modpackTotalHits = ref(0);
const modpackTotalPages = computed(() => Math.ceil(modpackTotalHits.value / modpackItemsPerPage));
const modpackInitialLoad = ref(true); // 初始加载状态

// 游戏版本列表（动态获取）
const gameVersions = ref<string[]>([]);

// 加载器列表
const loaders = ["fabric", "forge", "quilt", "neoforge"];

// 分类列表
const categories = [
  "adventure", "cursed", "decoration", "economy", "equipment",
  "food", "game-mechanics", "library", "magic", "optimization",
  "social", "storage", "technology", "transportation", "utility", "worldgen"
];

// 排序选项
const modpackSortOptions = [
  { title: "相关性", value: "relevance" },
  { title: "下载量", value: "downloads" },
  { title: "关注度", value: "follows" },
  { title: "最新", value: "newest" },
  { title: "更新", value: "updated" }
];



const filteredVersions = computed(() => {
  let filtered = versions.value.filter((version) => {
    const typeMatch =
      versionTypeFilter.value === "all" ||
      version.type === versionTypeFilter.value;
    const searchMatch =
      !searchVersion.value ||
      version.id.toLowerCase().includes(searchVersion.value.toLowerCase());
    return typeMatch && searchMatch;
  });

  if (sortOrder.value === "newest") {
    filtered.sort(
      (a, b) =>
        new Date(b.releaseTime).getTime() - new Date(a.releaseTime).getTime()
    );
  } else if (sortOrder.value === "oldest") {
    filtered.sort(
      (a, b) =>
        new Date(a.releaseTime).getTime() - new Date(b.releaseTime).getTime()
    );
  } else if (sortOrder.value === "az") {
    filtered.sort((a, b) => a.id.localeCompare(b.id));
  } else if (sortOrder.value === "za") {
    filtered.sort((a, b) => b.id.localeCompare(a.id));
  }

  return filtered;
});

const defaultInstanceName = computed(() => {
  if (selectedVersion.value) {
    return selectedVersion.value.id;
  }
  return "";
});

async function fetchVersions() {
  loadingVersions.value = true;
  try {
    const manifest = await invoke("get_versions");
    versions.value = (manifest as any).versions.map((v: any) => ({
      ...v,
      releaseTime: new Date(v.releaseTime).toLocaleString(),
    }));
    
    // 同时更新游戏版本列表（用于整合包过滤）
    gameVersions.value = (manifest as any).versions
      .filter((v: any) => v.type === "release") // 只包含正式版
      .map((v: any) => v.id)
      .sort((a: string, b: string) => {
        // 按版本号降序排列（最新在前）
        const aParts = a.split('.').map(Number);
        const bParts = b.split('.').map(Number);
        for (let i = 0; i < Math.max(aParts.length, bParts.length); i++) {
          const aPart = aParts[i] || 0;
          const bPart = bParts[i] || 0;
          if (aPart !== bPart) return bPart - aPart;
        }
        return 0;
      });
  } catch (error) {
    console.error("Failed to fetch versions:", error);
  } finally {
    loadingVersions.value = false;
  }
}

async function fetchModLoaderVersions() {
  if (
    !selectedVersion.value ||
    !selectedModLoaderType.value ||
    selectedModLoaderType.value === "None"
  ) {
    modLoaderVersions.value = [];
    return;
  }

  loadingModLoaderVersions.value = true;
  selectedModLoaderVersion.value = null;

  try {
    if (selectedModLoaderType.value === "Forge") {
      const result = await invoke("get_forge_versions", {
        minecraftVersion: selectedVersion.value.id,
      });
      modLoaderVersions.value = result as any[];
    } else {
      // Placeholder for other loaders like Fabric
      modLoaderVersions.value = [];
    }

    if (modLoaderVersions.value.length > 0) {
      selectedModLoaderVersion.value = modLoaderVersions.value[0];
    }
  } catch (error) {
    console.error(
      `Failed to fetch ${selectedModLoaderType.value} versions:`,
      error
    );
    modLoaderVersions.value = []; // Clear on error
  } finally {
    loadingModLoaderVersions.value = false;
  }
}

// 监听进度事件
let unlistenProgress: (() => void) | null = null;

async function createInstance() {
  if (!selectedVersion.value) {
    alert("请先选择一个Minecraft版本");
    return;
  }

  const finalInstanceName = instanceName.value || defaultInstanceName.value;
  if (!finalInstanceName) {
    alert("实例名称不能为空");
    return;
  }

  installing.value = true;
  showProgress.value = true;
  progressValue.value = 0;
  progressIndeterminate.value = true;
  progressText.value = "准备安装...";

  try {
    // 监听进度事件
    unlistenProgress = await listen(
      "instance-install-progress",
      (event: any) => {
        const progressData = event.payload;
        progressValue.value = progressData.progress;
        progressText.value = progressData.message;
        progressIndeterminate.value = progressData.indeterminate;
      }
    );

    let payload: any = {
      newInstanceName: finalInstanceName,
      baseVersionId: selectedVersion.value.id,
    };

    if (
      selectedModLoaderType.value === "Forge" &&
      selectedModLoaderVersion.value
    ) {
      payload.forgeVersion = selectedModLoaderVersion.value;
    }

    await invoke("create_instance", payload);

    alert(`实例 '${finalInstanceName}' 创建成功!`);

    // 重置进度状态
    showProgress.value = false;
    installing.value = false;
  } catch (error) {
    console.error("Failed to create instance:", error);
    progressText.value = "安装失败！";
    progressIndeterminate.value = false;
    installing.value = false;

    await new Promise((resolve) => setTimeout(resolve, 1000));
    showProgress.value = false;

    alert(`创建实例失败: ${error}`);
  } finally {
    // 取消监听
    if (unlistenProgress) {
      unlistenProgress();
      unlistenProgress = null;
    }
  }
}

onUnmounted(() => {
  // 组件卸载时取消监听
  if (unlistenProgress) {
    unlistenProgress();
  }
});

// 搜索整合包
async function searchModpacks() {
  console.log("开始搜索整合包...", {
    query: modpackSearchQuery.value,
    gameVersion: selectedGameVersion.value,
    loader: selectedLoader.value,
    category: selectedCategory.value,
    page: modpackCurrentPage.value,
    initialLoad: modpackInitialLoad.value
  });
  
  loadingModpacks.value = true;
  try {
    const offset = (modpackCurrentPage.value - 1) * modpackItemsPerPage;
    
    const result = await invoke("search_modrinth_modpacks", {
      query: modpackSearchQuery.value || undefined,
      gameVersions: selectedGameVersion.value ? [selectedGameVersion.value] : undefined,
      loaders: selectedLoader.value ? [selectedLoader.value] : undefined,
      categories: selectedCategory.value ? [selectedCategory.value] : undefined,
      limit: modpackItemsPerPage,
      offset: offset,
      sortBy: sortBy.value,
    }) as ModrinthSearchResult;
    
    modpacks.value = result.hits || [];
    modpackTotalHits.value = result.total_hits || 0;
    modpackInitialLoad.value = false; // 标记初始加载完成
    
    console.log("搜索结果:", {
      hits: result.hits?.length || 0,
      totalHits: result.total_hits || 0,
      modpacks: modpacks.value.length,
      initialLoad: modpackInitialLoad.value
    });
  } catch (error) {
    console.error("搜索整合包失败:", error);
    modpacks.value = [];
    modpackTotalHits.value = 0;
    modpackInitialLoad.value = false; // 即使出错也标记初始加载完成
  } finally {
    loadingModpacks.value = false;
  }
}

// 处理分页变化
async function onModpackPageChange(page: number) {
  modpackCurrentPage.value = page;
  await searchModpacks();
}

// 选择整合包
async function selectModpack(modpack: ModrinthModpack) {
  router.push({
    path: '/install-modpack',
    query: {
      projectId: modpack.slug,
      title: modpack.title
    }
  });
}

// 安装整合包
async function installModpack() {
  if (!selectedModpack.value || !selectedModpackVersion.value) {
    alert("请先选择整合包和版本");
    return;
  }

  const finalInstanceName = modpackInstanceName.value || selectedModpack.value.slug;
  if (!finalInstanceName) {
    alert("实例名称不能为空");
    return;
  }

  installingModpack.value = true;
  showModpackProgress.value = true;
  modpackProgressValue.value = 0;
  modpackProgressIndeterminate.value = true;
  modpackProgressText.value = "准备安装...";

  try {
    // 监听进度事件
    const unlistenProgress = await listen(
      "modpack-install-progress",
      (event: any) => {
        const progressData = event.payload;
        modpackProgressValue.value = progressData.progress;
        modpackProgressText.value = progressData.message;
        modpackProgressIndeterminate.value = progressData.indeterminate;
      }
    );

    const options = {
      modpack_id: selectedModpack.value.slug,
      version_id: selectedModpackVersion.value,
      instance_name: finalInstanceName,
      install_path: "", // 后端会自动处理路径
    };

    await invoke("install_modrinth_modpack", options);

    alert(`整合包 '${selectedModpack.value.title}' 安装成功!`);
    
    // 重置状态
    showModpackProgress.value = false;
    installingModpack.value = false;
    
    // 取消监听
    unlistenProgress();
  } catch (error) {
    console.error("安装整合包失败:", error);
    modpackProgressText.value = "安装失败！";
    modpackProgressIndeterminate.value = false;
    installingModpack.value = false;

    await new Promise((resolve) => setTimeout(resolve, 1000));
    showModpackProgress.value = false;

    alert(`安装整合包失败: ${error}`);
  }
}

// 格式化数字（下载量）
function formatNumber(num: number): string {
  if (num >= 1000000) {
    return (num / 1000000).toFixed(1) + 'M';
  } else if (num >= 1000) {
    return (num / 1000).toFixed(1) + 'K';
  }
  return num.toString();
}

// 截断文本
function truncateText(text: string, maxLength: number): string {
  if (!text) return '';
  if (text.length <= maxLength) return text;
  return text.substring(0, maxLength) + '...';
}

// 格式化版本范围
function formatVersionRange(versions: string[]): string {
  if (!versions || versions.length === 0) return '未知';
  if (versions.length <= 3) return versions.join(', ');
  
  // 按版本号排序
  const sortedVersions = [...versions].sort((a, b) => {
    const aParts = a.split('.').map(Number);
    const bParts = b.split('.').map(Number);
    for (let i = 0; i < Math.max(aParts.length, bParts.length); i++) {
      const aPart = aParts[i] || 0;
      const bPart = bParts[i] || 0;
      if (aPart !== bPart) return bPart - aPart; // 降序排列
    }
    return 0;
  });
  
  const latest = sortedVersions[0];
  const oldest = sortedVersions[sortedVersions.length - 1];
  
  if (latest === oldest) return latest;
  return `${latest} - ${oldest}`;
}

// 格式化加载器
function formatLoaders(loaders: string[]): string {
  if (!loaders || loaders.length === 0) return '未知';
  
  const loaderMap: { [key: string]: string } = {
    'fabric': 'Fabric',
    'forge': 'Forge',
    'quilt': 'Quilt',
    'neoforge': 'NeoForge'
  };
  
  return loaders.map(loader => loaderMap[loader] || loader).join(', ');
}

// 格式化日期
function formatDate(dateString: string): string {
  if (!dateString) return '未知';
  
  const date = new Date(dateString);
  const now = new Date();
  const diffTime = Math.abs(now.getTime() - date.getTime());
  const diffDays = Math.ceil(diffTime / (1000 * 60 * 60 * 24));
  
  if (diffDays === 1) return '昨天';
  if (diffDays < 7) return `${diffDays}天前`;
  if (diffDays < 30) return `${Math.floor(diffDays / 7)}周前`;
  if (diffDays < 365) return `${Math.floor(diffDays / 30)}月前`;
  return `${Math.floor(diffDays / 365)}年前`;
}

onMounted(() => {
  fetchVersions();
  // 自动加载热门整合包
  searchModpacks();
});

watch(selectedVersion, () => {
  selectedModLoaderType.value = "None";
  selectedModLoaderVersion.value = null;
  modLoaderVersions.value = [];
});

watch(selectedModLoaderType, () => {
  selectedModLoaderVersion.value = null;
  fetchModLoaderVersions();
});

// 监听安装类型变化
watch(installType, (newType) => {
  if (newType === 'online' && selectedPlatform.value === 'modrinth') {
    // 切换到从互联网安装时，自动搜索整合包
    searchModpacks();
  }
});

// 监听搜索条件变化，延迟搜索
let searchTimeout: number | null = null;
watch([modpackSearchQuery, selectedGameVersion, selectedLoader, selectedCategory, sortBy], () => {
  if (searchTimeout) {
    clearTimeout(searchTimeout);
  }
  // 重置到第一页
  modpackCurrentPage.value = 1;
  searchTimeout = setTimeout(() => {
    searchModpacks();
  }, 500);
});
</script>

<style scoped>
.install-type-tab {
  border-bottom: 2px solid transparent;
  transition: all 0.3s ease;
  color: rgba(var(--v-theme-on-surface), var(--v-medium-emphasis-opacity));
  font-weight: 500;
}

.install-type-tab:hover {
  box-shadow: 0 2px 4px -1px rgba(0, 0, 0, 0.1);
}

.install-type-active {
  border-bottom-color: rgb(var(--v-theme-primary));
  color: rgb(var(--v-theme-primary));
  font-weight: 600;
}

.install-type-divider {
  width: 1px;
  height: 24px;
  background-color: rgba(var(--v-theme-on-surface), 0.12);
  margin: 0 8px;
}

/* 移动端适配 */
@media (max-width: 600px) {
  .install-type-tab {
    font-size: 0.85rem;
    padding: 10px 6px;
  }

  .install-type-divider {
    height: 18px;
    margin: 0 4px;
  }

  /* 移动端输入框布局优化 */
  .v-row.no-gutters {
    flex-direction: column;
  }

  .v-row.no-gutters .v-col {
    max-width: 100% !important;
    margin-bottom: 16px;
  }

  .v-row.no-gutters .v-col:last-child {
    margin-bottom: 0;
  }
}

/* 深色模式适配 */
:deep(.v-theme--dark) .install-type-tab {
  color: rgba(255, 255, 255, 0.7);
}

:deep(.v-theme--dark) .install-type-tab:hover {
  box-shadow: 0 2px 4px -1px rgba(255, 255, 255, 0.1);
}

:deep(.v-theme--dark) .install-type-active {
  color: rgb(var(--v-theme-primary));
}

:deep(.v-theme--dark) .install-type-divider {
  background-color: rgba(255, 255, 255, 0.12);
}

/* 整合包卡片样式 */
.modpack-card {
  transition: all 0.3s ease;
  cursor: pointer;
}

.modpack-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
}

.modpack-card-selected {
  border: 2px solid rgb(var(--v-theme-primary));
  box-shadow: 0 4px 12px rgba(var(--v-theme-primary), 0.3);
}

.modpack-image {
  border-radius: 4px 4px 0 0;
}

.modpack-image-placeholder {
  height: 120px;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: rgba(var(--v-theme-surface-variant), 0.5);
  border-radius: 4px 4px 0 0;
}

.modpack-title {
  font-size: 1rem;
  font-weight: 600;
  line-height: 1.2;
  margin-bottom: 8px;
}

.modpack-info {
  font-size: 0.875rem;
  color: rgba(var(--v-theme-on-surface), 0.7);
}

.modpack-author {
  margin-bottom: 4px;
}

.modpack-downloads {
  margin-bottom: 4px;
}

.modpack-versions {
  margin-bottom: 4px;
}

.modpack-loaders {
  margin-bottom: 4px;
}

.modpack-categories {
  margin-bottom: 4px;
  font-size: 0.8rem;
  color: rgba(var(--v-theme-primary), 0.8);
}

.modpack-updated {
  margin-bottom: 8px;
  font-size: 0.8rem;
  color: rgba(var(--v-theme-on-surface), 0.5);
}

.modpack-description {
  font-size: 0.8rem;
  line-height: 1.4;
  color: rgba(var(--v-theme-on-surface), 0.6);
}
</style>
