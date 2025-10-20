<script setup lang="ts">
import { ref, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from '@tauri-apps/api/event';
import { useSettingsStore } from '../stores/settings';

const settingsStore = useSettingsStore()
const installedVersions = ref<string[]>([])
const selectedVersion = ref('')
const username = ref('') // Don't default to 'Player'
const offlineMode = ref(true)
const loading = ref(false)
const gameDir = ref('') // 游戏目录变量
const missingFiles = ref<string[]>([]); // 新增：缺失文件列表

// Load saved username from backend
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

// Save username to backend
async function saveUsername(newName: string) {
  try {
    await invoke('set_saved_username', { username: newName });
  } catch (err) {
    console.error("Failed to save username:", err);
  }
}

// 加载已保存的游戏目录
async function loadGameDir() {
  try {
    const dir = await invoke('get_game_dir');
    gameDir.value = dir as string;
    // 加载游戏目录后，获取已安装的版本
    await loadInstalledVersions();
  } catch (err) {
    console.error('Failed to get game directory:', err);
  }
}

// 获取已安装的版本
async function loadInstalledVersions() {
  try {
    loading.value = true;
    const dirInfo = await invoke('get_game_dir_info');
    if (dirInfo && (dirInfo as any).versions) {
      installedVersions.value = (dirInfo as any).versions;
      if (installedVersions.value.length > 0 && !selectedVersion.value) {
        selectedVersion.value = installedVersions.value[0]; // 默认选择第一个版本
      }
    }
    loading.value = false;
  } catch (err) {
    console.error('Failed to get installed versions:', err);
    loading.value = false;
  }
}

// 验证文件完整性
async function validateFiles() {
  if (!selectedVersion.value) {
    missingFiles.value = [];
    return;
  }
  try {
    loading.value = true;
    const result = await invoke('validate_version_files', { versionId: selectedVersion.value });
    missingFiles.value = result as string[];
  } catch (err) {
    console.error('Failed to validate version files:', err);
    missingFiles.value = [`验证文件失败: ${err}`];
  } finally {
    loading.value = false;
  }
}

// 启动游戏
async function launchGame() {
  if (!selectedVersion.value) {
    alert('请先选择一个版本');
    return;
  }
  
  // 预启动验证
  await validateFiles();
  if (missingFiles.value.length > 0) {
    alert('检测到缺失文件，请先下载或修复！\n' + missingFiles.value.join('\n'));
    return;
  }

  try {
    loading.value = true;
    await invoke('launch_minecraft', {
      options: {
        version: selectedVersion.value,
        memory: settingsStore.maxMemory,
        username: username.value,
        offline: offlineMode.value,
        game_dir: gameDir.value
      }
    });
    loading.value = false;
  } catch (err) {
    console.error('Failed to launch game:', err);
    loading.value = false;
    alert(`启动失败: ${err}`);
  }
}

// 在组件挂载时加载游戏目录和监听事件
onMounted(async () => {
  await loadGameDir();
  await loadUsername();
  await validateFiles(); // 初始验证
  
  // 监听游戏目录变更事件
  await listen('game-dir-changed', (event) => {
    gameDir.value = event.payload as string;
    // 游戏目录变更后，重新加载已安装的版本并验证
    loadInstalledVersions();
    validateFiles();
  });
});

// 监听 selectedVersion 变化进行验证
watch(selectedVersion, (newVal) => {
  if (newVal) {
    validateFiles();
  } else {
    missingFiles.value = [];
  }
});

// Watch for username changes and save them
watch(username, (newName) => {
  if (newName !== null && newName !== undefined) {
    saveUsername(newName);
  }
});
</script>

<template>
  <v-container>
    <v-row>
      <!-- 左侧：版本选择和设置区域 -->
      <v-col cols="12" md="7" lg="8">
        <!-- 版本选择卡片 -->
        <v-card class="mb-4">
          <v-card-title class="d-flex align-center">
            <v-icon start>mdi-play-circle</v-icon>
            版本选择
            <v-spacer></v-spacer>
            <v-btn variant="text" icon="mdi-refresh" @click="loadInstalledVersions" :loading="loading"></v-btn>
          </v-card-title>
          <v-card-text>
            <v-select
              v-model="selectedVersion"
              :items="installedVersions"
              label="选择已安装的游戏版本"
              :loading="loading"
              :hint="installedVersions.length === 0 ? '没有找到已安装的版本，请先在下载页面下载' : ''"
              persistent-hint
            ></v-select>
            
            <v-alert
              v-if="missingFiles.length > 0"
              type="warning"
              class="mb-4"
              closable
            >
              <div class="d-flex align-center">
                <span>检测到游戏文件缺失，请前往下载页面下载</span>
              </div>
            </v-alert>

            <div class="d-flex justify-end">
              <v-btn
                variant="text"
                color="primary"
                prepend-icon="mdi-download"
                to="/download"
                size="small"
              >
                下载新版本
              </v-btn>
            </div>
          </v-card-text>
        </v-card>

        <!-- 游戏设置卡片 -->
        <v-card>
          <v-card-title>
            <v-icon start>mdi-cog</v-icon>
            游戏设置
          </v-card-title>
          <v-card-text>
            <v-row>
              <v-col cols="12" sm="6">
                <v-text-field
                  v-model="username"
                  label="用户名"
                  hide-details
                ></v-text-field>
              </v-col>
              <v-col cols="12" sm="6">
                <v-switch
                  v-model="offlineMode"
                  label="离线模式"
                  color="primary"
                  hide-details
                ></v-switch>
              </v-col>
            </v-row>
          </v-card-text>
        </v-card>
      </v-col>

      <!-- 右侧：启动游戏和快速导航 -->
      <v-col cols="12" md="5" lg="4">
        <!-- 启动游戏卡片 -->
        <v-card class="mb-4">
          <v-card-title class="text-center">
            启动游戏
          </v-card-title>
          <v-card-text class="text-center">
            <v-btn
              block
              color="primary"
              size="large"
              :loading="loading"
              :disabled="!selectedVersion || missingFiles.length > 0"
              @click="launchGame"
              class="mb-2"
            >
              <v-icon start>mdi-play</v-icon>
              启动 Minecraft
            </v-btn>
            
            <div class="text-caption text-medium-emphasis">
              <v-icon size="16" class="mr-1">mdi-information</v-icon>
              {{ !selectedVersion ? '请先选择游戏版本' : missingFiles.length > 0 ? '需要下载缺失文件' : '准备就绪' }}
            </div>
          </v-card-text>
        </v-card>

        <!-- 快速导航卡片 -->
        <v-card>
          <v-card-title>快速导航</v-card-title>
          <v-card-text>
            <v-list density="compact">
              <v-list-item to="/add-instance" prepend-icon="mdi-plus">
                <v-list-item-title>添加实例</v-list-item-title>
              </v-list-item>
              <v-list-item to="/instance-manager" prepend-icon="mdi-folder">
                <v-list-item-title>实例管理</v-list-item-title>
              </v-list-item>
              <v-list-item to="/download" prepend-icon="mdi-download">
                <v-list-item-title>下载中心</v-list-item-title>
              </v-list-item>
            </v-list>
          </v-card-text>
        </v-card>
      </v-col>
    </v-row>
  </v-container>
</template>

<style scoped>
/* 响应式布局调整 */
@media (max-width: 960px) {
  .v-container {
    padding: 16px;
  }
}

@media (max-width: 800px) {
  .v-col-md-5,
  .v-col-md-7 {
    flex: 0 0 100%;
    max-width: 100%;
  }
}

/* 深色模式适配 */
:deep(.v-theme--dark) .v-card {
  background-color: #1e1e1e;
}
</style>