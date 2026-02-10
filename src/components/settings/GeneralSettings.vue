<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { useSettingsStore } from '../../stores/settings';
import { configApi } from '../../services';
import { logError } from '../../utils/logger';
import pkg from '../../../package.json';

const settingsStore = useSettingsStore();
const gameDir = ref('');
const versionIsolation = ref(true);
const downloadThreads = ref(32);
const isolateSaves = ref(true);
const isolateResourcepacks = ref(true);
const isolateLogs = ref(true);
const appVersion = pkg.version || '0.0.0';
const themePalette = ref(localStorage.getItem('themePalette') || 'indigo');
const themePalettes = [
  { title: 'MD3 经典紫', value: 'indigo' },
  { title: 'MD3 清新青', value: 'emerald' },
  { title: 'MD3 玫瑰', value: 'rose' },
];

let unlistenGameDirChanged: UnlistenFn | null = null;

async function loadGameDir() {
  try {
    gameDir.value = await configApi.getGameDir();
  } catch (err) {
    logError('Failed to get game directory', err, 'GeneralSettings')
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
      await configApi.setGameDir(gameDir.value);
    }
  } catch (err) {
    logError('Failed to select directory', err, 'GeneralSettings')
  }
}

async function loadDownloadThreads() {
  try {
    downloadThreads.value = await configApi.getDownloadThreads();
  } catch (err) {
    logError('Failed to get download threads', err, 'GeneralSettings')
  }
}

async function saveDownloadThreads() {
  try {
    await configApi.setDownloadThreads(downloadThreads.value);
  } catch (err) {
    logError('Failed to set download threads', err, 'GeneralSettings')
  }
}

async function loadVersionIsolation() {
  try {
    const isolation = await configApi.loadConfigKey('versionIsolation');
    if (isolation !== null) {
      versionIsolation.value = isolation === 'true';
    }
    
    const saves = await configApi.loadConfigKey('isolateSaves');
    if (saves !== null) {
      isolateSaves.value = saves === 'true';
    }
    
    const resourcepacks = await configApi.loadConfigKey('isolateResourcepacks');
    if (resourcepacks !== null) {
      isolateResourcepacks.value = resourcepacks === 'true';
    }
    
    const logs = await configApi.loadConfigKey('isolateLogs');
    if (logs !== null) {
      isolateLogs.value = logs === 'true';
    }
  } catch (err) {
    logError('Failed to load isolation settings', err, 'GeneralSettings')
  }
}

async function saveIsolationSetting(key: string, value: boolean) {
  try {
    await configApi.saveConfigKey(key, value.toString());
  } catch (err) {
    logError(`Failed to save ${key}`, err, 'GeneralSettings')
  }
}

watch(versionIsolation, (v) => saveIsolationSetting('versionIsolation', v));
watch(isolateSaves, (v) => saveIsolationSetting('isolateSaves', v));
watch(isolateResourcepacks, (v) => saveIsolationSetting('isolateResourcepacks', v));
watch(isolateLogs, (v) => saveIsolationSetting('isolateLogs', v));

watch(() => settingsStore.downloadMirror, async () => {
  await settingsStore.saveDownloadMirror();
});

watch(themePalette, (value) => {
  localStorage.setItem('themePalette', value);
  window.dispatchEvent(new CustomEvent('theme-palette-changed', { detail: value }));
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
    <div class="group-header mb-5">
      <div class="d-flex align-center">
        <v-avatar size="48" color="primary-container" class="mr-3">
          <v-icon size="24" color="on-primary-container">mdi-cog-outline</v-icon>
        </v-avatar>
        <div>
          <h2 class="text-h6 font-weight-bold">常规设置</h2>
          <p class="text-body-2 text-on-surface-variant mb-0">游戏目录和下载配置</p>
        </div>
      </div>
    </div>

    <!-- 游戏目录 -->
    <v-card color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-3">
          <v-icon class="mr-2" color="on-surface-variant">mdi-folder-outline</v-icon>
          <span class="text-subtitle-1 font-weight-medium">游戏目录</span>
        </div>
        <v-text-field
          v-model="gameDir"
          placeholder="选择游戏安装目录"
          readonly
          hide-details
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
    <v-card color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center justify-space-between mb-1">
          <div class="d-flex align-center">
            <v-icon class="mr-2" color="on-surface-variant">mdi-folder-multiple-outline</v-icon>
            <span class="text-subtitle-1 font-weight-medium">版本隔离</span>
          </div>
          <v-switch
            v-model="versionIsolation"
            hide-details
            density="compact"
            color="primary"
          />
        </div>
        <p class="text-body-2 text-on-surface-variant mb-0">
          为每个游戏版本创建独立的文件夹，避免配置冲突
        </p>

        <!-- 隔离选项 -->
        <v-expand-transition>
          <div v-if="versionIsolation" class="mt-4 pt-4 isolation-options">
            <div class="text-body-2 text-on-surface-variant mb-3">选择需要隔离的内容：</div>
            <v-row dense>
              <v-col cols="12" sm="4">
                <v-checkbox
                  v-model="isolateSaves"
                  label="存档"
                  density="compact"
                  hide-details
                  color="primary"
                />
              </v-col>
              <v-col cols="12" sm="4">
                <v-checkbox
                  v-model="isolateResourcepacks"
                  label="资源包"
                  density="compact"
                  hide-details
                  color="primary"
                />
              </v-col>
              <v-col cols="12" sm="4">
                <v-checkbox
                  v-model="isolateLogs"
                  label="日志"
                  density="compact"
                  hide-details
                  color="primary"
                />
              </v-col>
            </v-row>
          </div>
        </v-expand-transition>
      </v-card-text>
    </v-card>

    <!-- 下载设置 -->
    <v-card color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-4">
          <v-icon class="mr-2" color="on-surface-variant">mdi-download-outline</v-icon>
          <span class="text-subtitle-1 font-weight-medium">下载设置</span>
        </div>

        <!-- 下载线程 -->
        <div class="mb-5">
          <div class="d-flex align-center mb-2">
            <span class="text-body-2">下载线程数</span>
            <v-chip size="small" color="primary" variant="tonal" class="ml-2">{{ downloadThreads }}</v-chip>
          </div>
          <v-slider
            v-model="downloadThreads"
            :min="1"
            :max="64"
            :step="1"
            hide-details
            color="primary"
            @end="saveDownloadThreads"
          >
            <template #append>
              <span class="text-caption text-on-surface-variant">64</span>
            </template>
          </v-slider>
        </div>

        <!-- 下载源 -->
        <div>
          <div class="text-body-2 mb-3">下载源</div>
          <v-btn-toggle
            v-model="settingsStore.downloadMirror"
            mandatory
            density="comfortable"
            divided
            color="primary"
          >
            <v-btn value="official" class="px-4">
              <v-icon start size="18">mdi-web</v-icon>
              官方源
            </v-btn>
            <v-btn value="bmcl" class="px-4">
              <v-icon start size="18">mdi-lightning-bolt</v-icon>
              BMCL 镜像
            </v-btn>
          </v-btn-toggle>
          <p class="text-caption text-on-surface-variant mt-2 mb-0">
            BMCL 镜像通常在国内访问更快
          </p>
        </div>
      </v-card-text>
    </v-card>

    <!-- 外观 -->
    <v-card color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-4">
          <v-icon class="mr-2" color="on-surface-variant">mdi-palette-outline</v-icon>
          <span class="text-subtitle-1 font-weight-medium">外观</span>
        </div>
        <v-select
          v-model="themePalette"
          :items="themePalettes"
          item-title="title"
          item-value="value"
          label="主题色方案（MD3）"
          hide-details
        />
        <div class="text-caption text-on-surface-variant mt-2">
          主题色方案会在浅色/深色模式之间保持一致的 MD3 色调风格
        </div>
      </v-card-text>
    </v-card>

    <!-- 关于 -->
    <v-card color="surface-container">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-4">
          <v-icon class="mr-2" color="on-surface-variant">mdi-information-outline</v-icon>
          <span class="text-subtitle-1 font-weight-medium">关于</span>
        </div>
        <div class="d-flex align-center justify-space-between mb-2">
          <div class="text-body-2 text-on-surface-variant">应用版本</div>
          <v-chip size="small" color="primary" variant="tonal">{{ appVersion }}</v-chip>
        </div>
        <div class="text-caption text-on-surface-variant">
          快捷键：Ctrl/⌘ + D 下载，Ctrl/⌘ + N 新建实例，Ctrl/⌘ + I 实例管理，Ctrl/⌘ + , 设置
        </div>
      </v-card-text>
    </v-card>
  </div>
</template>

<style scoped>
.isolation-options {
  border-top: 1px solid rgb(var(--v-theme-outline-variant));
}
</style>
