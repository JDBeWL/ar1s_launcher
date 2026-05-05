<script setup lang="ts">
import { ref, onMounted, watch, computed } from "vue";
import { VStepperVertical, VStepperVerticalItem } from "vuetify/labs/components";
import { useVersionManager } from "../composables/useVersionManager";
import { useGameLaunch } from "../composables/useGameLaunch";
import { useAuthStore } from "../stores/authStore";
import { instanceApi } from "../services";
import { formatTimeAgo, formatLastPlayed } from "../utils/format";
import { useDebounceFn } from "../composables/useDebounce";
import { logError } from "../utils/logger";
import type { GameInstance } from "../types/events";

const {
  installedVersions,
  selectedVersion,
  loading: versionLoading,
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

const authStore = useAuthStore()
const pageLoading = ref(true)

const isReady = computed(() => {
  if (!selectedVersion.value) return false
  return authStore.isLoggedIn
})

const RECENT_PLAY_KEY = 'minecraft_recent_plays'
const MAX_RECENT = 3

interface RecentPlay {
  version: string
  timestamp: number
}

const recentPlays = ref<RecentPlay[]>([])

const instances = ref<GameInstance[]>([])
const recentInstances = computed(() => {
  return [...instances.value]
    .sort((a, b) => (b.lastPlayed ?? 0) - (a.lastPlayed ?? 0))
    .slice(0, 3)
})

function loadRecentPlays() {
  try {
    const saved = localStorage.getItem(RECENT_PLAY_KEY)
    if (saved) {
      recentPlays.value = JSON.parse(saved)
    }
  } catch (e) {
    logError('Failed to load recent plays', e, 'HomeView')
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

const instanceCount = ref(0)

async function loadInstanceCount() {
  try {
    const list = await instanceApi.getInstances()
    instances.value = list || []
    instanceCount.value = instances.value.length
  } catch (e) {
    logError('Failed to load instances', e, 'HomeView')
  }
}

const debouncedSaveUsername = useDebounceFn((name: string) => {
  authStore.saveUsername(name);
}, 500);

watch(() => authStore.username, (newName) => {
  if (newName !== null && newName !== undefined) {
    debouncedSaveUsername.call(newName);
  }
});

async function handleLaunch() {
  if (selectedVersion.value) {
    saveRecentPlay(selectedVersion.value)
  }

  if (authStore.authType === 'microsoft' && authStore.msLoggedIn) {
    // Token 过期时尝试刷新，失败仍使用旧 token
    if (authStore.isTokenExpired) {
      await authStore.tryRefreshMicrosoftToken()
    }
    await launchGame(selectedVersion.value, authStore.msUsername, {
      authType: 'microsoft',
      accessToken: authStore.msAccessToken,
      uuid: authStore.msUuid,
    })
  } else {
    await launchGame(selectedVersion.value, authStore.username)
  }
}

onMounted(async () => {
  loadRecentPlays();
  const isRevisit = instances.value.length > 0;
  await Promise.all([
    loadGameDir(),
    authStore.init(),
    initListeners(),
    isRevisit ? Promise.resolve() : loadInstanceCount()
  ]);
  pageLoading.value = false;
});
</script>

<template>
  <v-container fluid class="page-container pa-4">
    <!-- 加载骨架屏 -->
    <template v-if="pageLoading">
      <v-row>
        <v-col cols="12" md="7">
          <div class="d-flex flex-column ga-3">
            <v-card color="surface-container" variant="flat">
              <v-card-text class="pa-5">
                <div class="d-flex align-center mb-5">
                  <v-skeleton-loader type="avatar" class="mr-4" />
                  <div>
                    <v-skeleton-loader type="text" style="width: 120px;" />
                    <v-skeleton-loader type="text" style="width: 180px;" class="mt-1" />
                  </div>
                </div>
                <v-skeleton-loader type="text" style="width: 100%;" class="mb-3" />
                <v-skeleton-loader type="text" style="width: 100%;" class="mb-3" />
                <v-skeleton-loader type="button" style="width: 100%; height: 48px;" />
              </v-card-text>
            </v-card>
            <v-card color="surface-container" variant="flat">
              <v-card-text class="pa-4">
                <v-skeleton-loader type="text" style="width: 80px;" class="mb-3" />
                <div class="d-flex ga-2 mb-4">
                  <v-skeleton-loader type="chip" style="width: 80px;" />
                  <v-skeleton-loader type="chip" style="width: 80px;" />
                </div>
                <v-divider class="mb-3" />
                <div class="d-flex ga-4">
                  <v-skeleton-loader type="text" style="width: 60px;" />
                  <v-skeleton-loader type="text" style="width: 60px;" />
                </div>
              </v-card-text>
            </v-card>
          </div>
        </v-col>
        <v-col cols="12" md="5">
          <div class="d-flex flex-column ga-3">
            <v-card color="surface-container" variant="flat">
              <v-card-text class="pa-3">
                <v-skeleton-loader type="list-item-two-line" />
                <v-skeleton-loader type="list-item-two-line" />
                <v-skeleton-loader type="list-item-two-line" />
              </v-card-text>
            </v-card>
            <v-card color="surface-container" variant="flat">
              <v-card-text class="pa-3">
                <v-skeleton-loader type="button" style="width: 100%;" class="mb-2" />
                <v-skeleton-loader type="button" style="width: 100%;" />
              </v-card-text>
            </v-card>
          </div>
        </v-col>
      </v-row>
    </template>

    <template v-else>
      <v-row>
        <!-- 左侧：启动区域 -->
        <v-col cols="12" md="7">
          <div class="d-flex flex-column ga-3 h-100">
            <v-card color="surface-container" variant="flat" class="launch-card">
              <v-card-text class="pa-5">
                <div class="d-flex align-center mb-4">
                  <v-avatar size="48" color="primary">
                    <v-icon size="24" color="on-primary">mdi-minecraft</v-icon>
                  </v-avatar>
                  <div class="ml-3">
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
                  class="mb-4"
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

                <!-- 认证状态行 -->
                <div class="auth-status-row mb-4">
                  <div class="d-flex align-center">
                    <v-avatar size="32" :color="isReady ? 'primary-container' : 'surface-container-high'" class="mr-2">
                      <v-icon size="16" :color="isReady ? 'on-primary-container' : 'on-surface-variant'">
                        {{ authStore.authType === 'microsoft' ? 'mdi-microsoft' : 'mdi-account' }}
                      </v-icon>
                    </v-avatar>
                    <div class="flex-grow-1">
                      <template v-if="authStore.authType === 'microsoft'">
                        <div v-if="authStore.msLoggedIn" class="text-body-2 font-weight-medium">{{ authStore.msUsername }}</div>
                        <div v-else class="text-body-2 text-on-surface-variant">未登录</div>
                      </template>
                      <template v-else>
                        <div v-if="authStore.username" class="text-body-2 font-weight-medium">{{ authStore.username }}</div>
                        <div v-else class="text-body-2 text-on-surface-variant">未设置玩家名称</div>
                      </template>
                    </div>
                    <v-btn size="small" variant="text" to="/settings">
                      <v-icon size="16" class="mr-1">mdi-cog</v-icon>
                      认证设置
                    </v-btn>
                  </div>
                </div>

                <!-- 启动按钮 -->
                <v-btn
                  block
                  size="x-large"
                  :color="isReady ? 'primary' : 'surface-container-high'"
                  :loading="launchLoading"
                  :disabled="!isReady"
                  class="launch-btn"
                  @click="handleLaunch"
                >
                  <v-icon start size="24">mdi-play</v-icon>
                  启动游戏
                </v-btn>

                <!-- 未就绪时的引导提示 -->
                <div v-if="!isReady" class="launch-guide mt-3">
                  <v-stepper-vertical hide-actions flat>
                    <v-stepper-vertical-item
                      :complete="!!selectedVersion"
                      :color="selectedVersion ? 'success' : 'warning'"
                      :icon="selectedVersion ? 'mdi-check' : 'mdi-cube-outline'"
                      title="选择游戏版本"
                      value="version"
                    >
                      <div class="text-caption text-on-surface-variant">从上方下拉菜单选择一个已安装的版本</div>
                    </v-stepper-vertical-item>
                    <v-stepper-vertical-item
                      :complete="!!authStore.displayName"
                      :color="authStore.displayName ? 'success' : 'warning'"
                      :icon="authStore.displayName ? 'mdi-check' : 'mdi-account-outline'"
                      :title="authStore.authType === 'microsoft' ? '登录 Microsoft 账户' : '设置玩家名称'"
                      value="auth"
                    >
                      <div class="text-caption text-on-surface-variant">
                        前往 <router-link to="/settings" class="text-primary">设置页面</router-link> 配置认证信息
                      </div>
                    </v-stepper-vertical-item>
                  </v-stepper-vertical>
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

        <!-- 右侧：信息与快捷操作 -->
        <v-col cols="12" md="5">
          <div class="d-flex flex-column ga-3">
            <v-card color="surface-container" variant="flat">
              <v-card-text class="pa-3 d-flex align-center">
                <div class="d-flex align-center flex-grow-1">
                  <v-icon size="18" class="mr-2" color="on-surface-variant">mdi-clock-outline</v-icon>
                  <span class="text-body-2 font-weight-medium">最近实例</span>
                </div>
                <v-btn size="x-small" variant="text" to="/instance-manager">
                  管理
                </v-btn>
              </v-card-text>
              <v-divider />
              <v-card-text class="pa-3">
                <div v-if="recentInstances.length === 0" class="text-caption text-on-surface-variant">
                  暂无实例，创建后会在这里显示
                </div>
                <v-list v-else density="compact" bg-color="transparent">
                  <v-list-item
                    v-for="instance in recentInstances"
                    :key="instance.id"
                    class="px-0"
                  >
                    <template #prepend>
                      <v-avatar size="32" color="primary-container" class="mr-2">
                        <v-icon size="16" color="on-primary-container">mdi-cube-outline</v-icon>
                      </v-avatar>
                    </template>
                    <v-list-item-title class="text-body-2">
                      {{ instance.name }}
                    </v-list-item-title>
                    <v-list-item-subtitle class="text-caption">
                      {{ instance.gameVersion || instance.version }} · {{ formatLastPlayed(instance.lastPlayed) }}
                    </v-list-item-subtitle>
                    <template #append>
                      <v-btn 
                        size="x-small" 
                        variant="text" 
                        :to="{ path: '/instance-manager', query: { search: instance.name } }"
                      >
                        查看
                      </v-btn>
                    </template>
                  </v-list-item>
                </v-list>
              </v-card-text>
            </v-card>

            <v-card color="surface-container" variant="flat">
              <v-card-text class="pa-3">
                <div class="d-flex align-center mb-3">
                  <v-icon size="18" class="mr-2" color="on-surface-variant">mdi-lightning-bolt</v-icon>
                  <span class="text-body-2 font-weight-medium">快捷操作</span>
                </div>
                <div class="d-flex flex-column ga-2">
                  <v-btn variant="tonal" color="primary" block to="/download">
                    <v-icon start size="18">mdi-download</v-icon>
                    下载版本
                  </v-btn>
                  <v-btn variant="tonal" color="secondary" block to="/add-instance">
                    <v-icon start size="18">mdi-plus</v-icon>
                    创建实例
                  </v-btn>
                </div>
              </v-card-text>
            </v-card>
          </div>
        </v-col>
      </v-row>
    </template>

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
              <span>{{ repairProgress.speed < 1024 ? repairProgress.speed.toFixed(0) + ' KB/s' : (repairProgress.speed / 1024).toFixed(1) + ' MB/s' }}</span>
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
.launch-btn {
  font-weight: 700;
  font-size: 1.1rem;
  letter-spacing: 0.5px;
  height: 52px !important;
}

.launch-btn:not(:disabled) {
  box-shadow: 0 4px 16px rgba(var(--v-theme-primary), 0.35);
}

.launch-card {
  border: 1px solid rgb(var(--v-theme-outline-variant));
}

.auth-status-row {
  padding: 8px 12px;
  border-radius: 12px;
  background-color: rgb(var(--v-theme-surface-container-high));
}

.recent-chip {
  cursor: pointer;
}

.stat-item {
  text-align: center;
  flex: 1;
}

.launch-guide :deep(.v-stepper-vertical) {
  background: transparent;
  padding: 0;
}

.launch-guide :deep(.v-stepper-vertical-item__title) {
  font-size: 0.875rem;
}
</style>
