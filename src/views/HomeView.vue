<script setup lang="ts">
import { ref, onMounted, watch, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useVersionManager } from "../composables/useVersionManager";
import { useGameLaunch } from "../composables/useGameLaunch";

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

const isReady = computed(() => selectedVersion.value && username.value)

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
});
</script>

<template>
  <v-container fluid class="home-container pa-4">
    <!-- 顶部欢迎区域 -->
    <div class="welcome-section mb-5">
      <h1 class="text-h5 font-weight-bold mb-1">欢迎回来</h1>
      <p class="text-body-2 text-on-surface-variant">准备好开始你的 Minecraft 冒险了吗？</p>
    </div>

    <v-row>
      <!-- 左侧主要区域 -->
      <v-col cols="12" md="8">
        <!-- 启动卡片 -->
        <v-card color="surface-container-low" class="launch-card mb-4">
          <v-card-text class="pa-5">
            <v-row align="center" no-gutters>
              <v-col cols="12" sm="7" class="pr-sm-4">
                <div class="d-flex align-center mb-4">
                  <v-avatar size="48" color="primary-container" class="mr-3">
                    <v-icon size="24" color="on-primary-container">mdi-minecraft</v-icon>
                  </v-avatar>
                  <div>
                    <div class="text-subtitle-1 font-weight-bold">启动游戏</div>
                    <div class="text-body-2 text-on-surface-variant">
                      {{ selectedVersion ? `已选择: ${selectedVersion}` : '请选择游戏版本' }}
                    </div>
                  </div>
                </div>

                <!-- 版本选择 -->
                <v-select
                  v-model="selectedVersion"
                  :items="installedVersions"
                  :loading="versionLoading"
                  label="游戏版本"
                  hide-details
                  class="mb-3"
                >
                  <template #prepend-inner>
                    <v-icon size="20" color="on-surface-variant">mdi-gamepad-variant</v-icon>
                  </template>
                  <template #append>
                    <v-btn
                      icon
                      variant="text"
                      size="x-small"
                      :loading="versionLoading"
                      @click.stop="loadInstalledVersions"
                    >
                      <v-icon size="18">mdi-refresh</v-icon>
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

                <!-- 用户名输入 -->
                <v-text-field
                  v-model="username"
                  label="玩家名称"
                  hide-details
                  placeholder="输入你的游戏名称"
                >
                  <template #prepend-inner>
                    <v-icon size="20" color="on-surface-variant">mdi-account</v-icon>
                  </template>
                </v-text-field>
              </v-col>

              <v-col cols="12" sm="5" class="d-flex flex-column align-center justify-center py-4 py-sm-0">
                <v-btn
                  size="large"
                  color="primary"
                  :loading="launchLoading"
                  :disabled="!isReady"
                  elevation="0"
                  class="launch-btn px-10"
                  @click="handleLaunch"
                >
                  <v-icon start size="24">mdi-play</v-icon>
                  启动
                </v-btn>
                <div class="text-caption text-on-surface-variant mt-3">
                  <v-icon size="14" class="mr-1" :color="isReady ? 'success' : 'on-surface-variant'">
                    {{ isReady ? 'mdi-check-circle' : 'mdi-information' }}
                  </v-icon>
                  {{ isReady ? '准备就绪' : '请填写版本和玩家名称' }}
                </div>
              </v-col>
            </v-row>
          </v-card-text>
        </v-card>

        <!-- 快捷操作 -->
        <v-row dense>
          <v-col cols="4">
            <v-card
              color="surface-container"
              class="quick-action-card"
              to="/download"
            >
              <v-card-text class="pa-4 text-center">
                <v-avatar size="48" color="primary-container" class="mb-3">
                  <v-icon size="24" color="on-primary-container">mdi-download</v-icon>
                </v-avatar>
                <div class="text-body-2 font-weight-medium">下载版本</div>
                <div class="text-caption text-on-surface-variant d-none d-sm-block">获取新的游戏版本</div>
              </v-card-text>
            </v-card>
          </v-col>
          <v-col cols="4">
            <v-card
              color="surface-container"
              class="quick-action-card"
              to="/add-instance"
            >
              <v-card-text class="pa-4 text-center">
                <v-avatar size="48" color="primary-container" class="mb-3">
                  <v-icon size="24" color="on-primary-container">mdi-plus-circle</v-icon>
                </v-avatar>
                <div class="text-body-2 font-weight-medium">添加实例</div>
                <div class="text-caption text-on-surface-variant d-none d-sm-block">创建自定义游戏实例</div>
              </v-card-text>
            </v-card>
          </v-col>
          <v-col cols="4">
            <v-card
              color="surface-container"
              class="quick-action-card"
              to="/instance-manager"
            >
              <v-card-text class="pa-4 text-center">
                <v-avatar size="48" color="primary-container" class="mb-3">
                  <v-icon size="24" color="on-primary-container">mdi-folder-multiple</v-icon>
                </v-avatar>
                <div class="text-body-2 font-weight-medium">实例管理</div>
                <div class="text-caption text-on-surface-variant d-none d-sm-block">管理已有的游戏实例</div>
              </v-card-text>
            </v-card>
          </v-col>
        </v-row>
      </v-col>

      <!-- 右侧信息区域 -->
      <v-col cols="12" md="4">
        <!-- 游戏设置 -->
        <v-card color="surface-container" class="mb-4">
          <v-card-text class="pa-4">
            <div class="d-flex align-center mb-4">
              <v-avatar size="40" color="primary-container" class="mr-3">
                <v-icon size="20" color="on-primary-container">mdi-cog</v-icon>
              </v-avatar>
              <div class="text-body-1 font-weight-medium">游戏设置</div>
            </div>

            <div class="d-flex align-center justify-space-between py-2">
              <div class="d-flex align-center">
                <v-icon size="20" class="mr-2" color="on-surface-variant">mdi-wifi-off</v-icon>
                <span class="text-body-2">离线模式</span>
              </div>
              <v-switch
                v-model="offlineMode"
                hide-details
                density="compact"
                color="primary"
              />
            </div>

            <v-divider class="my-3" />

            <v-btn
              variant="tonal"
              color="secondary"
              block
              size="small"
              to="/settings"
            >
              <v-icon start size="18">mdi-tune</v-icon>
              更多设置
            </v-btn>
          </v-card-text>
        </v-card>

        <!-- 状态信息 -->
        <v-card color="surface-container">
          <v-card-text class="pa-4">
            <div class="d-flex align-center mb-4">
              <v-avatar size="40" color="primary-container" class="mr-3">
                <v-icon size="20" color="on-primary-container">mdi-information</v-icon>
              </v-avatar>
              <div class="text-body-1 font-weight-medium">状态信息</div>
            </div>

            <div class="status-item d-flex align-center justify-space-between py-2">
              <span class="text-body-2 text-on-surface-variant">已安装版本</span>
              <v-chip size="small" color="primary" variant="tonal">
                {{ installedVersions.length }}
              </v-chip>
            </div>

            <v-divider class="my-2" />

            <div class="status-item d-flex align-center justify-space-between py-2">
              <span class="text-body-2 text-on-surface-variant">游戏目录</span>
              <v-tooltip :text="gameDir" location="top">
                <template #activator="{ props }">
                  <v-chip
                    v-bind="props"
                    size="small"
                    :color="gameDir ? 'success' : 'warning'"
                    variant="tonal"
                    class="text-truncate"
                    style="max-width: 100px"
                  >
                    {{ gameDir ? '已设置' : '未设置' }}
                  </v-chip>
                </template>
              </v-tooltip>
            </div>

            <v-divider class="my-2" />

            <div class="status-item d-flex align-center justify-space-between py-2">
              <span class="text-body-2 text-on-surface-variant">登录状态</span>
              <v-chip size="small" :color="offlineMode ? 'secondary' : 'success'" variant="tonal">
                {{ offlineMode ? '离线' : '在线' }}
              </v-chip>
            </div>
          </v-card-text>
        </v-card>
      </v-col>
    </v-row>

    <!-- 修复进度对话框 -->
    <v-dialog :model-value="isRepairing" persistent max-width="420">
      <v-card color="surface-container-high">
        <v-card-text class="pa-6">
          <div class="text-center mb-5">
            <v-avatar size="72" color="primary-container" class="mb-4">
              <v-icon size="36" color="on-primary-container">mdi-wrench</v-icon>
            </v-avatar>
            <div class="text-h6 font-weight-bold">正在修复游戏文件</div>
          </div>

          <div v-if="repairProgress">
            <div class="d-flex justify-space-between mb-2">
              <span class="text-body-2">
                {{ repairProgress.status === 'downloading' ? '下载中...' : '处理中...' }}
              </span>
              <span class="text-body-2 font-weight-medium">{{ repairProgress.percent }}%</span>
            </div>
            <v-progress-linear
              :model-value="repairProgress.percent"
              height="8"
              rounded
              color="primary"
            />
            <div class="d-flex justify-space-between mt-2 text-caption text-on-surface-variant">
              <span>{{ (repairProgress.bytes_downloaded / 1024 / 1024).toFixed(2) }} MB</span>
              <span>{{ (repairProgress.speed / 1024).toFixed(1) }} KB/s</span>
            </div>
          </div>
          <div v-else class="text-center py-4">
            <v-progress-circular indeterminate size="48" color="primary" />
            <div class="mt-3 text-body-2 text-on-surface-variant">准备中...</div>
          </div>
        </v-card-text>
      </v-card>
    </v-dialog>
  </v-container>
</template>

<style scoped>
.home-container {
  max-width: 1000px;
  margin: 0 auto;
}

.welcome-section {
  padding-left: 4px;
}

.launch-btn {
  min-width: 140px;
  min-height: 52px;
  font-size: 1rem;
  font-weight: 600;
  letter-spacing: 0.5px;
}

.quick-action-card {
  cursor: pointer;
  transition: transform 0.2s cubic-bezier(0.4, 0, 0.2, 1), 
              box-shadow 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

.quick-action-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
}
</style>
