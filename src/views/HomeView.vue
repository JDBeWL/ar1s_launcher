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
    <div class="welcome-section mb-4">
      <h1 class="text-h5 font-weight-bold mb-1">欢迎回来</h1>
      <p class="text-body-2 text-medium-emphasis">准备好开始你的 Minecraft 冒险了吗？</p>
    </div>

    <v-row>
      <!-- 左侧主要区域 -->
      <v-col cols="12" md="8">
        <!-- 启动卡片 -->
        <v-card variant="outlined" rounded="xl" class="launch-card mb-4">
          <v-card-text class="pa-5">
            <v-row align="center" no-gutters>
              <v-col cols="12" sm="7" class="pr-sm-4">
                <div class="d-flex align-center mb-4">
                  <v-avatar size="44" class="mr-3 avatar-outlined">
                    <v-icon size="22">mdi-minecraft</v-icon>
                  </v-avatar>
                  <div>
                    <div class="text-subtitle-1 font-weight-bold">启动游戏</div>
                    <div class="text-body-2 text-medium-emphasis">
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
                  variant="outlined"
                  density="compact"
                  rounded="lg"
                  hide-details
                  class="mb-3"
                >
                  <template #prepend-inner>
                    <v-icon size="20">mdi-gamepad-variant</v-icon>
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
                      <v-list-item-title class="text-medium-emphasis">
                        没有已安装的版本
                      </v-list-item-title>
                    </v-list-item>
                  </template>
                </v-select>

                <!-- 用户名输入 -->
                <v-text-field
                  v-model="username"
                  label="玩家名称"
                  variant="outlined"
                  density="compact"
                  rounded="lg"
                  hide-details
                  placeholder="输入你的游戏名称"
                >
                  <template #prepend-inner>
                    <v-icon size="20">mdi-account</v-icon>
                  </template>
                </v-text-field>
              </v-col>

              <v-col cols="12" sm="5" class="d-flex flex-column align-center justify-center py-4 py-sm-0">
                <v-btn
                  size="large"
                  rounded="xl"
                  :loading="launchLoading"
                  :disabled="!isReady"
                  elevation="2"
                  class="launch-btn px-8"
                  @click="handleLaunch"
                >
                  <v-icon start size="24">mdi-play</v-icon>
                  启动
                </v-btn>
                <div class="text-caption text-medium-emphasis mt-2">
                  <v-icon size="12" class="mr-1">
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
              variant="outlined"
              rounded="xl"
              class="quick-action-card"
              to="/download"
            >
              <v-card-text class="pa-3 text-center">
                <v-avatar size="40" class="mb-2 avatar-outlined">
                  <v-icon size="20">mdi-download</v-icon>
                </v-avatar>
                <div class="text-body-2 font-weight-medium">下载版本</div>
                <div class="text-caption text-medium-emphasis d-none d-sm-block">获取新的游戏版本</div>
              </v-card-text>
            </v-card>
          </v-col>
          <v-col cols="4">
            <v-card
              variant="outlined"
              rounded="xl"
              class="quick-action-card"
              to="/add-instance"
            >
              <v-card-text class="pa-3 text-center">
                <v-avatar size="40" class="mb-2 avatar-outlined">
                  <v-icon size="20">mdi-plus-circle</v-icon>
                </v-avatar>
                <div class="text-body-2 font-weight-medium">添加实例</div>
                <div class="text-caption text-medium-emphasis d-none d-sm-block">创建自定义游戏实例</div>
              </v-card-text>
            </v-card>
          </v-col>
          <v-col cols="4">
            <v-card
              variant="outlined"
              rounded="xl"
              class="quick-action-card"
              to="/instance-manager"
            >
              <v-card-text class="pa-3 text-center">
                <v-avatar size="40" class="mb-2 avatar-outlined">
                  <v-icon size="20">mdi-folder-multiple</v-icon>
                </v-avatar>
                <div class="text-body-2 font-weight-medium">实例管理</div>
                <div class="text-caption text-medium-emphasis d-none d-sm-block">管理已有的游戏实例</div>
              </v-card-text>
            </v-card>
          </v-col>
        </v-row>
      </v-col>

      <!-- 右侧信息区域 -->
      <v-col cols="12" md="4">
        <!-- 游戏设置 -->
        <v-card variant="outlined" rounded="xl" class="mb-4">
          <v-card-text class="pa-4">
            <div class="d-flex align-center mb-3">
              <v-avatar size="36" class="mr-3 avatar-outlined">
                <v-icon size="18">mdi-cog</v-icon>
              </v-avatar>
              <div class="text-body-1 font-weight-medium">游戏设置</div>
            </div>

            <div class="d-flex align-center justify-space-between py-2">
              <div class="d-flex align-center">
                <v-icon size="18" class="mr-2">mdi-wifi-off</v-icon>
                <span class="text-body-2">离线模式</span>
              </div>
              <v-switch
                v-model="offlineMode"
                hide-details
                density="compact"
              />
            </div>

            <v-divider class="my-2" />

            <v-btn
              variant="outlined"
              block
              rounded="lg"
              size="small"
              to="/settings"
              class="mt-2"
            >
              <v-icon start size="16">mdi-tune</v-icon>
              更多设置
            </v-btn>
          </v-card-text>
        </v-card>

        <!-- 状态信息 -->
        <v-card variant="outlined" rounded="xl">
          <v-card-text class="pa-4">
            <div class="d-flex align-center mb-3">
              <v-avatar size="36" class="mr-3 avatar-outlined">
                <v-icon size="18">mdi-information</v-icon>
              </v-avatar>
              <div class="text-body-1 font-weight-medium">状态信息</div>
            </div>

            <div class="status-item d-flex align-center justify-space-between py-2">
              <span class="text-body-2 text-medium-emphasis">已安装版本</span>
              <v-chip size="x-small" variant="outlined">
                {{ installedVersions.length }}
              </v-chip>
            </div>

            <v-divider class="my-1" />

            <div class="status-item d-flex align-center justify-space-between py-2">
              <span class="text-body-2 text-medium-emphasis">游戏目录</span>
              <v-tooltip :text="gameDir" location="top">
                <template #activator="{ props }">
                  <v-chip
                    v-bind="props"
                    size="x-small"
                    variant="outlined"
                    class="text-truncate"
                    style="max-width: 100px"
                  >
                    {{ gameDir ? '已设置' : '未设置' }}
                  </v-chip>
                </template>
              </v-tooltip>
            </div>

            <v-divider class="my-1" />

            <div class="status-item d-flex align-center justify-space-between py-2">
              <span class="text-body-2 text-medium-emphasis">登录状态</span>
              <v-chip size="x-small" variant="outlined">
                {{ offlineMode ? '离线' : '在线' }}
              </v-chip>
            </div>
          </v-card-text>
        </v-card>
      </v-col>
    </v-row>

    <!-- 修复进度对话框 -->
    <v-dialog :model-value="isRepairing" persistent max-width="420">
      <v-card rounded="xl">
        <v-card-text class="pa-6">
          <div class="text-center mb-4">
            <v-avatar size="64" class="mb-3 avatar-outlined">
              <v-icon size="32">mdi-wrench</v-icon>
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
            />
            <div class="d-flex justify-space-between mt-2 text-caption text-medium-emphasis">
              <span>{{ (repairProgress.bytes_downloaded / 1024 / 1024).toFixed(2) }} MB</span>
              <span>{{ (repairProgress.speed / 1024).toFixed(1) }} KB/s</span>
            </div>
          </div>
          <div v-else class="text-center py-4">
            <v-progress-circular indeterminate size="48" />
            <div class="mt-3 text-body-2 text-medium-emphasis">准备中...</div>
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

.launch-card {
  border-width: 1px;
}

.launch-btn {
  min-width: 120px;
  min-height: 48px;
  font-size: 1rem;
  font-weight: 600;
  transition: transform 0.2s ease, box-shadow 0.2s ease;
}

.launch-btn:hover:not(:disabled) {
  transform: translateY(-2px);
}

.quick-action-card {
  cursor: pointer;
  transition: transform 0.2s ease, border-color 0.2s ease;
}

.quick-action-card:hover {
  transform: translateY(-3px);
}

.avatar-outlined {
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
}
</style>
