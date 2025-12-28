<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { useSettingsStore } from '../../stores/settings';

const settingsStore = useSettingsStore();
const gameDir = ref('');
const versionIsolation = ref(true);
const downloadThreads = ref(32);
const isolateSaves = ref(true);
const isolateResourcepacks = ref(true);
const isolateLogs = ref(true);

let unlistenGameDirChanged: UnlistenFn | null = null;

async function loadGameDir() {
  try {
    const dir = await invoke('get_game_dir');
    gameDir.value = dir as string;
  } catch (err) {
    console.error('Failed to get game directory:', err);
  }
}

async function selectGameDir() {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: '选择游戏目录'
    });
    if (selected) {
      gameDir.value = selected as string;
      await invoke('set_game_dir', { path: gameDir.value, window: {} });
    }
  } catch (err) {
    console.error('Failed to select directory:', err);
  }
}

async function loadDownloadThreads() {
  try {
    const threads = await invoke('get_download_threads');
    downloadThreads.value = threads as number;
  } catch (err) {
    console.error('Failed to get download threads:', err);
  }
}

async function saveDownloadThreads() {
  try {
    await invoke('set_download_threads', { threads: downloadThreads.value });
  } catch (err) {
    console.error('Failed to set download threads:', err);
  }
}

async function loadVersionIsolation() {
  try {
    const isolation = await invoke('load_config_key', { key: 'versionIsolation' });
    versionIsolation.value = isolation === 'true';
    
    const saves = await invoke('load_config_key', { key: 'isolateSaves' });
    isolateSaves.value = saves === 'true';
    
    const resourcepacks = await invoke('load_config_key', { key: 'isolateResourcepacks' });
    isolateResourcepacks.value = resourcepacks === 'true';
    
    const logs = await invoke('load_config_key', { key: 'isolateLogs' });
    isolateLogs.value = logs === 'true';
  } catch (err) {
    console.error('Failed to load isolation settings:', err);
  }
}

async function saveIsolationSetting(key: string, value: boolean) {
  try {
    await invoke('save_config_key', { key, value: value.toString() });
  } catch (err) {
    console.error(`Failed to save ${key}:`, err);
  }
}

watch(versionIsolation, (v) => saveIsolationSetting('versionIsolation', v));
watch(isolateSaves, (v) => saveIsolationSetting('isolateSaves', v));
watch(isolateResourcepacks, (v) => saveIsolationSetting('isolateResourcepacks', v));
watch(isolateLogs, (v) => saveIsolationSetting('isolateLogs', v));

watch(() => settingsStore.downloadMirror, async () => {
  await settingsStore.saveDownloadMirror();
});

onMounted(async () => {
  await loadGameDir();
  await loadDownloadThreads();
  await loadVersionIsolation();
  await settingsStore.loadDownloadMirror();
  
  unlistenGameDirChanged = await listen('game-dir-changed', (event) => {
    gameDir.value = event.payload as string;
  });
});

onUnmounted(() => {
  if (unlistenGameDirChanged) {
    unlistenGameDirChanged();
    unlistenGameDirChanged = null;
  }
});
</script>

<template>
  <div class="settings-group">
    <!-- 标题 -->
    <div class="group-header mb-4">
      <div class="d-flex align-center">
        <v-avatar color="primary" variant="tonal" size="40" class="mr-3">
          <v-icon>mdi-cog-outline</v-icon>
        </v-avatar>
        <div>
          <h2 class="text-h6 font-weight-bold">常规设置</h2>
          <p class="text-body-2 text-medium-emphasis mb-0">游戏目录和下载配置</p>
        </div>
      </div>
    </div>

    <!-- 游戏目录 -->
    <v-card variant="outlined" rounded="lg" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-3">
          <v-icon color="primary" class="mr-2">mdi-folder-outline</v-icon>
          <span class="text-subtitle-1 font-weight-medium">游戏目录</span>
        </div>
        <v-text-field
          v-model="gameDir"
          variant="outlined"
          density="comfortable"
          placeholder="选择游戏安装目录"
          readonly
          hide-details
          rounded="lg"
        >
          <template #append-inner>
            <v-btn
              icon
              variant="text"
              size="small"
              @click="selectGameDir"
            >
              <v-icon>mdi-folder-open-outline</v-icon>
            </v-btn>
          </template>
        </v-text-field>
      </v-card-text>
    </v-card>

    <!-- 版本隔离 -->
    <v-card variant="outlined" rounded="lg" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center justify-space-between mb-1">
          <div class="d-flex align-center">
            <v-icon color="primary" class="mr-2">mdi-folder-multiple-outline</v-icon>
            <span class="text-subtitle-1 font-weight-medium">版本隔离</span>
          </div>
          <v-switch
            v-model="versionIsolation"
            color="primary"
            hide-details
            density="compact"
          />
        </div>
        <p class="text-body-2 text-medium-emphasis mb-0">
          为每个游戏版本创建独立的文件夹，避免配置冲突
        </p>

        <!-- 隔离选项 -->
        <v-expand-transition>
          <div v-if="versionIsolation" class="mt-4 pt-4" style="border-top: 1px solid rgba(var(--v-border-color), var(--v-border-opacity))">
            <div class="text-body-2 text-medium-emphasis mb-3">选择需要隔离的内容：</div>
            <v-row dense>
              <v-col cols="12" sm="4">
                <v-checkbox
                  v-model="isolateSaves"
                  label="存档"
                  density="compact"
                  hide-details
                />
              </v-col>
              <v-col cols="12" sm="4">
                <v-checkbox
                  v-model="isolateResourcepacks"
                  label="资源包"
                  density="compact"
                  hide-details
                />
              </v-col>
              <v-col cols="12" sm="4">
                <v-checkbox
                  v-model="isolateLogs"
                  label="日志"
                  density="compact"
                  hide-details
                />
              </v-col>
            </v-row>
          </div>
        </v-expand-transition>
      </v-card-text>
    </v-card>

    <!-- 下载设置 -->
    <v-card variant="outlined" rounded="lg">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-4">
          <v-icon color="primary" class="mr-2">mdi-download-outline</v-icon>
          <span class="text-subtitle-1 font-weight-medium">下载设置</span>
        </div>

        <!-- 下载线程 -->
        <div class="mb-6">
          <div class="d-flex align-center justify-space-between mb-2">
            <span class="text-body-2">下载线程数</span>
            <v-chip size="small" color="primary" variant="tonal">{{ downloadThreads }}</v-chip>
          </div>
          <v-slider
            v-model="downloadThreads"
            :min="1"
            :max="64"
            :step="1"
            color="primary"
            track-color="grey-lighten-2"
            hide-details
            @end="saveDownloadThreads"
          >
            <template #prepend>
              <span class="text-caption text-medium-emphasis">1</span>
            </template>
            <template #append>
              <span class="text-caption text-medium-emphasis">64</span>
            </template>
          </v-slider>
        </div>

        <!-- 下载源 -->
        <div>
          <div class="text-body-2 mb-3">下载源</div>
          <v-btn-toggle
            v-model="settingsStore.downloadMirror"
            mandatory
            rounded="lg"
            density="comfortable"
            color="primary"
            variant="outlined"
          >
            <v-btn value="official" class="px-4">
              <v-icon start>mdi-web</v-icon>
              官方源
            </v-btn>
            <v-btn value="bmcl" class="px-4">
              <v-icon start>mdi-lightning-bolt</v-icon>
              BMCL 镜像
            </v-btn>
          </v-btn-toggle>
          <p class="text-caption text-medium-emphasis mt-2 mb-0">
            BMCL 镜像通常在国内访问更快
          </p>
        </div>
      </v-card-text>
    </v-card>
  </div>
</template>

<style scoped>
.settings-group {
  margin-bottom: 32px;
}

.group-header {
  padding-bottom: 16px;
  border-bottom: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
}
</style>
