<script setup lang="ts">
import { ref, onMounted, watch, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useVersionManager } from "../composables/useVersionManager";
import { useGameLaunch } from "../composables/useGameLaunch";
import { formatTimeAgo } from "../utils/format";

const {
  installedVersions,
  selectedVersion,
  loading: versionLoading,
  gameDir,
  loadGameDir,
  loadInstalledVersions,
  initListeners
} = useVersionManager();

const { 
  loading: launchLoading, 
  launchGame,
  isRepairing,
  repairProgress 
} = useGameLaunch();

const username = ref('')
const offlineMode = ref(true)

const isReady = computed(() => selectedVersion.value && username.value && username.value.trim())

// 最近游玩记录
const RECENT_PLAY_KEY = 'minecraft_recent_plays'
const MAX_RECENT = 3

interface RecentPlay {
  version: string
  timestamp: number
}

const recentPlays = ref<RecentPlay[]>([])

function loadRecentPlays() {
  try {
    const saved = localStorage.getItem(RECENT_PLAY_KEY)
    if (saved) {
      recentPlays.value = JSON.parse(saved)
    }
  } catch (e) {
    console.error('Failed to load recent plays:', e)
  }
}

function saveRecentPlay(version: string) {
  const now = Date.now()
  const filtered = recentPlays.value.filter(p => p.version !== version)
  filtered.unshift({ version, timestamp: now })
  recentPlays.value = filtered.slice(0, MAX_RECENT)
  localStorage.setItem(RECENT_PLAY_KEY, JSON.stringify(recentPlays.value))
}

function quickLaunch(version: string) {
  selectedVersion.value = version
  if (isReady.value) {
    handleLaunch()
  }
}

// 实例统计
const instanceCount = ref(0)

async function loadInstanceCount() {
  try {
    const instances = await invoke('get_instances') as any[]
    instanceCount.value = instances?.length || 0
  } catch (e) {
    console.error('Failed to load instances:', e)
  }
}

async function loadUsername() {
  try {
    const savedUsername = await invoke('get_saved_username');
    if (savedUsername) {
      username.value = savedUsername as string;
    }
  } catch (err) {
    console.error("Failed to load username:", err);
  }
}

async function saveUsername(newName: string) {
  try {
    await invoke('set_saved_username', { username: newName });
  } catch (err) {
    console.error("Failed to save username:", err);
  }
}

watch(username, (newName) => {
  if (newName !== null && newName !== undefined) {
    saveUsername(newName);
  }
});

async function handleLaunch() {
  // 记录最近游玩
  if (selectedVersion.value) {
    saveRecentPlay(selectedVersion.value)
  }
  await launchGame(
    selectedVersion.value,
    username.value,
    offlineMode.value,
    gameDir.value
  );
}

onMounted(async () => {
  await loadGameDir();
  await loadUsername();
  await initListeners();
  loadRecentPlays();
  await loadInstanceCount();
});
</script>

<template>
  <v-container fluid class="home-container pa-4">
    <v-row>
      <!-- 左侧：启动区域 -->
      <v-col cols="12" md="7">
        <div class="d-flex flex-column ga-3 h-100">
          <v-card color="surface-container" variant="flat">
            <v-card-text class="pa-5">
            <!-- 标题 -->
            <div class="d-flex align-center mb-5">
              <v-avatar size="52" color="primary">
                <v-icon size="26" color="on-primary">mdi-minecraft</v-icon>
              </v-avatar>
              <div class="ml-4">
                <div class="text-h6 font-weight-bold">启动游戏</div>
                <div class="text-body-2 text-on-surface-variant">
                  {{ selectedVersion || '选择版本开始游戏' }}
                </div>
              </div>
            </div>

            <!-- 版本选择 -->
            <v-select
              v-model="selectedVersion"
              :items="installedVersions"
              :loading="versionLoading"
              label="游戏版本"
              density="comfortable"
              variant="outlined"
              hide-details
              class="mb-3"
            >
              <template #prepend-inner>
                <v-icon size="18" color="on-surface-variant">mdi-gamepad-variant</v-icon>
              </template>
              <template #append-inner>
                <v-btn
                  icon
                  variant="text"
                  size="x-small"
                  :loading="versionLoading"
                  @click.stop="loadInstalledVersions"
                >
                  <v-icon size="16">mdi-refresh</v-icon>
                </v-btn>
              </template>
              <template #no-data>
                <v-list-item>
                  <v-list-item-title class="text-on-surface-variant">
                    没有已安装的版本
                  </v-list-item-title>
                </v-list-item>
              </template>
            </v-select>

            <!-- 玩家名称 -->
            <v-text-field
              v-model="username"
              label="玩家名称"
              density="comfortable"
              variant="outlined"
              hide-details
              placeholder="输入游戏名称"
              autocomplete="off"
              class="mb-3"
            >
              <template #prepend-inner>
                <v-icon size="18" color="on-surface-variant">mdi-account</v-icon>
              </template>
            </v-text-field>

            <!-- 模式选择 -->
            <div class="d-flex align-center justify-space-between mb-5">
              <div class="d-flex align-center">
                <v-icon size="18" color="on-surface-variant" class="mr-2">mdi-wifi-off</v-icon>
                <span class="text-body-2">离线模式</span>
              </div>
              <v-switch
                v-model="offlineMode"
                hide-details
                density="compact"
                color="primary"
              />
            </div>

            <!-- 启动按钮 -->
            <v-btn
              block
              size="large"
              color="primary"
              :loading="launchLoading"
              :disabled="!isReady"
              class="launch-btn"
              @click="handleLaunch"
            >
              <v-icon start size="22">mdi-play</v-icon>
              启动游戏
            </v-btn>

            <div class="text-center text-caption text-on-surface-variant mt-3">
              <v-icon size="12" class="mr-1" :color="isReady ? 'success' : 'on-surface-variant'">
                {{ isReady ? 'mdi-check-circle' : 'mdi-information' }}
              </v-icon>
              {{ isReady ? '准备就绪' : '请选择版本并填写玩家名称' }}
            </div>
          </v-card-text>
        </v-card>

        <!-- 最近游玩 & 统计 -->
        <v-card color="surface-container" variant="flat">
          <v-card-text class="pa-4">
            <div class="d-flex align-center mb-3">
              <v-icon size="18" color="on-surface-variant" class="mr-2">mdi-history</v-icon>
              <span class="text-body-2 font-weight-medium">最近游玩</span>
            </div>
            
            <template v-if="recentPlays.length > 0">
              <div class="d-flex flex-wrap ga-2 mb-4">
                <v-chip
                  v-for="play in recentPlays"
                  :key="play.version"
                  size="small"
                  variant="tonal"
                  color="primary"
                  class="recent-chip"
                  @click="quickLaunch(play.version)"
                >
                  <v-icon start size="14">mdi-minecraft</v-icon>
                  {{ play.version }}
                  <v-tooltip activator="parent" location="top">
                    {{ formatTimeAgo(play.timestamp) }}
                  </v-tooltip>
                </v-chip>
              </div>
            </template>
            <div v-else class="text-caption text-on-surface-variant mb-4">
              暂无游玩记录，启动游戏后会在这里显示
            </div>

            <v-divider class="mb-3" />

            <!-- 统计信息 -->
            <div class="d-flex ga-4">
              <div class="stat-item">
                <div class="text-h6 font-weight-bold text-primary">{{ installedVersions.length }}</div>
                <div class="text-caption text-on-surface-variant">已安装版本</div>
              </div>
              <div class="stat-item">
                <div class="text-h6 font-weight-bold text-primary">{{ instanceCount }}</div>
                <div class="text-caption text-on-surface-variant">游戏实例</div>
              </div>
            </div>
          </v-card-text>
        </v-card>
        </div>
      </v-col>

      <!-- 右侧：快捷操作 -->
      <v-col cols="12" md="5">
        <div class="d-flex flex-column ga-3">
          <v-card
            color="surface-container"
            variant="flat"
            class="quick-action-card"
            to="/download"
          >
            <v-card-text class="pa-3 d-flex align-center">
              <v-avatar size="40" color="primary-container" class="mr-3">
                <v-icon size="20" color="on-primary-container">mdi-download</v-icon>
              </v-avatar>
              <div>
                <div class="text-body-2 font-weight-medium">下载版本</div>
                <div class="text-caption text-on-surface-variant">获取新的游戏版本</div>
              </div>
              <v-spacer />
              <v-icon size="20" color="on-surface-variant">mdi-chevron-right</v-icon>
            </v-card-text>
          </v-card>

          <v-card
            color="surface-container"
            variant="flat"
            class="quick-action-card"
            to="/add-instance"
          >
            <v-card-text class="pa-3 d-flex align-center">
              <v-avatar size="40" color="primary-container" class="mr-3">
                <v-icon size="20" color="on-primary-container">mdi-plus-circle</v-icon>
              </v-avatar>
              <div>
                <div class="text-body-2 font-weight-medium">添加实例</div>
                <div class="text-caption text-on-surface-variant">创建自定义游戏实例</div>
              </div>
              <v-spacer />
              <v-icon size="20" color="on-surface-variant">mdi-chevron-right</v-icon>
            </v-card-text>
          </v-card>

          <v-card
            color="surface-container"
            variant="flat"
            class="quick-action-card"
            to="/instance-manager"
          >
            <v-card-text class="pa-3 d-flex align-center">
              <v-avatar size="40" color="primary-container" class="mr-3">
                <v-icon size="20" color="on-primary-container">mdi-folder-multiple</v-icon>
              </v-avatar>
              <div>
                <div class="text-body-2 font-weight-medium">实例管理</div>
                <div class="text-caption text-on-surface-variant">管理已有的游戏实例</div>
              </div>
              <v-spacer />
              <v-icon size="20" color="on-surface-variant">mdi-chevron-right</v-icon>
            </v-card-text>
          </v-card>

          <v-card
            color="surface-container"
            variant="flat"
            class="quick-action-card"
            to="/settings"
          >
            <v-card-text class="pa-3 d-flex align-center">
              <v-avatar size="40" color="primary-container" class="mr-3">
                <v-icon size="20" color="on-primary-container">mdi-cog</v-icon>
              </v-avatar>
              <div>
                <div class="text-body-2 font-weight-medium">设置</div>
                <div class="text-caption text-on-surface-variant">配置启动器选项</div>
              </div>
              <v-spacer />
              <v-icon size="20" color="on-surface-variant">mdi-chevron-right</v-icon>
            </v-card-text>
          </v-card>
        </div>
      </v-col>
    </v-row>

    <!-- 修复进度对话框 -->
    <v-dialog :model-value="isRepairing" persistent max-width="380">
      <v-card color="surface-container-high">
        <v-card-text class="pa-5">
          <div class="text-center mb-4">
            <v-progress-circular
              v-if="!repairProgress"
              indeterminate
              size="44"
              color="primary"
            />
            <v-avatar v-else size="52" color="primary-container">
              <v-icon size="26" color="on-primary-container">mdi-wrench</v-icon>
            </v-avatar>
          </div>
          <div class="text-h6 font-weight-bold text-center mb-4">修复游戏文件</div>

          <template v-if="repairProgress">
            <div class="d-flex justify-space-between mb-2 text-body-2">
              <span>{{ repairProgress.status === 'downloading' ? '下载中' : '处理中' }}</span>
              <span class="font-weight-medium">{{ repairProgress.percent }}%</span>
            </div>
            <v-progress-linear
              :model-value="repairProgress.percent"
              height="6"
              rounded
              color="primary"
            />
            <div class="d-flex justify-space-between mt-2 text-caption text-on-surface-variant">
              <span>{{ (repairProgress.bytes_downloaded / 1024 / 1024).toFixed(1) }} MB</span>
              <span>{{ (repairProgress.speed / 1024).toFixed(0) }} KB/s</span>
            </div>
          </template>
          <div v-else class="text-center text-body-2 text-on-surface-variant">
            准备中...
          </div>
        </v-card-text>
      </v-card>
    </v-dialog>
  </v-container>
</template>

<style scoped>
.home-container {
  max-width: 900px;
  margin: 0 auto;
}

.launch-btn {
  font-weight: 600;
  font-size: 1rem;
}

.quick-action-card {
  cursor: pointer;
  transition: box-shadow 0.15s ease;
  min-height: 64px;
}

.quick-action-card:hover {
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
}

.recent-chip {
  cursor: pointer;
}

.stat-item {
  text-align: center;
  flex: 1;
}
</style>
