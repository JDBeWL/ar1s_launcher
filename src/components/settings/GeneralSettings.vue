<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue';
import { open } from '@tauri-apps/plugin-dialog';
import { listen } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { useSettingsStore } from '../../stores/settings';
import { configApi } from '../../services';
import { logError } from '../../utils/logger';
import { getThemePalette } from '../../utils/storage';

const settingsStore = useSettingsStore();
const gameDir = ref('');
const versionIsolation = ref(true);
const downloadThreads = ref(32);
const isolateSaves = ref(true);
const isolateResourcepacks = ref(true);
const isolateLogs = ref(true);
const appVersion = __APP_VERSION__;
const themePalette = ref(getThemePalette());
const themePalettes = [
  { title: 'MD3 经典紫', value: 'indigo' },
  { title: 'MD3 清新青', value: 'emerald' },
  { title: 'MD3 玫瑰', value: 'rose' },
];

const gameLanguage = ref('zh_cn');
const gameLanguages = [
  { title: '简体中文', value: 'zh_cn' },
  { title: '繁體中文', value: 'zh_tw' },
  { title: 'English (US)', value: 'en_us' },
  { title: 'English (UK)', value: 'en_gb' },
  { title: '日本語', value: 'ja_jp' },
  { title: '한국어', value: 'ko_kr' },
  { title: 'Français', value: 'fr_fr' },
  { title: 'Deutsch', value: 'de_de' },
  { title: 'Español', value: 'es_es' },
  { title: 'Italiano', value: 'it_it' },
  { title: 'Português (BR)', value: 'pt_br' },
  { title: 'Русский', value: 'ru_ru' },
];

let unlistenGameDirChanged: UnlistenFn | null = null;

async function loadGameDir() {
  try {
    gameDir.value = await configApi.getGameDir();
  } catch (err) {
    logError('Failed to get game directory', err, 'GeneralSettings');
  }
}

async function selectGameDir() {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: '选择游戏目录',
    });
    if (selected) {
      gameDir.value = selected as string;
      await configApi.setGameDir(gameDir.value);
    }
  } catch (err) {
    logError('Failed to select directory', err, 'GeneralSettings');
  }
}

async function loadDownloadThreads() {
  try {
    downloadThreads.value = await configApi.getDownloadThreads();
  } catch (err) {
    logError('Failed to get download threads', err, 'GeneralSettings');
  }
}

async function saveDownloadThreads() {
  try {
    await configApi.setDownloadThreads(downloadThreads.value);
  } catch (err) {
    logError('Failed to set download threads', err, 'GeneralSettings');
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
    logError('Failed to load isolation settings', err, 'GeneralSettings');
  }
}

async function saveIsolationSetting(key: string, value: boolean) {
  try {
    await configApi.saveConfigKey(key, value.toString());
  } catch (err) {
    logError(`Failed to save ${key}`, err, 'GeneralSettings');
  }
}

watch(versionIsolation, (v) => saveIsolationSetting('versionIsolation', v));
watch(isolateSaves, (v) => saveIsolationSetting('isolateSaves', v));
watch(isolateResourcepacks, (v) => saveIsolationSetting('isolateResourcepacks', v));
watch(isolateLogs, (v) => saveIsolationSetting('isolateLogs', v));
watch(gameLanguage, async (v) => {
  try {
    await configApi.saveConfigKey('language', v);
  } catch (err) {
    logError('Failed to save language', err, 'GeneralSettings');
  }
});

watch(
  () => settingsStore.downloadMirror,
  async () => {
    await settingsStore.saveDownloadMirror();
  },
);

watch(themePalette, (value) => {
  localStorage.setItem('themePalette', value);
  window.dispatchEvent(new CustomEvent('theme-palette-changed', { detail: value }));
  // 落盘交给 App.vue 的 theme-palette-changed 处理，它同时写配置和缓存
});

onMounted(async () => {
  await loadGameDir();
  await loadDownloadThreads();
  await loadVersionIsolation();
  await settingsStore.loadSettings();

  try {
    const lang = await configApi.loadConfigKey('language');
    if (lang) gameLanguage.value = lang;
  } catch (err) {
    logError('Failed to load language', err, 'GeneralSettings');
  }

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
    <!-- 游戏目录 -->
    <v-card variant="flat" color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-3">
          <v-icon size="20" class="mr-2" color="on-surface-variant">mdi-folder-outline</v-icon>
          <span class="text-title-medium">游戏目录</span>
        </div>
        <v-text-field density="comfortable" v-model="gameDir" label="游戏安装目录" readonly hide-details>
          <template #append-inner>
            <v-btn icon variant="text" size="x-small" aria-label="选择游戏目录" @click="selectGameDir">
              <v-icon size="18">mdi-folder-open-outline</v-icon>
            </v-btn>
          </template>
        </v-text-field>
      </v-card-text>
    </v-card>

    <!-- 版本隔离 -->
    <v-card variant="flat" color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center justify-space-between mb-1">
          <div class="d-flex align-center">
            <v-icon size="20" class="mr-2" color="on-surface-variant">mdi-folder-multiple-outline</v-icon>
            <span class="text-title-medium">版本隔离</span>
          </div>
          <v-switch
            v-model="versionIsolation"
            aria-label="版本隔离"
            hide-details
            inset
            density="compact"
            color="primary"
          />
        </div>
        <p class="text-body-medium text-on-surface-variant mb-0">为每个游戏版本创建独立的文件夹，避免配置冲突</p>

        <!-- 隔离选项 -->
        <v-expand-transition>
          <div v-if="versionIsolation" class="mt-4 pt-4 isolation-options">
            <div class="text-body-medium text-on-surface-variant mb-3">选择需要隔离的内容：</div>
            <div class="d-flex flex-wrap ga-6 isolation-checks">
              <v-checkbox v-model="isolateSaves" label="存档" density="compact" hide-details color="primary" />
              <v-checkbox
                v-model="isolateResourcepacks"
                label="资源包"
                density="compact"
                hide-details
                color="primary"
              />
              <v-checkbox v-model="isolateLogs" label="日志" density="compact" hide-details color="primary" />
            </div>
          </div>
        </v-expand-transition>
      </v-card-text>
    </v-card>

    <!-- 下载设置 -->
    <v-card variant="flat" color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-4">
          <v-icon size="20" class="mr-2" color="on-surface-variant">mdi-download-outline</v-icon>
          <span class="text-title-medium">下载设置</span>
        </div>

        <!-- 下载线程 -->
        <div class="mb-5">
          <div class="d-flex align-center mb-2">
            <span class="text-body-medium">下载线程数</span>
            <v-chip size="small" color="primary" variant="tonal" class="ml-2">{{ downloadThreads }}</v-chip>
          </div>
          <v-slider
            v-model="downloadThreads"
            :min="1"
            :max="64"
            :step="1"
            hide-details
            thumb-label
            color="primary"
            @end="saveDownloadThreads"
          >
            <template #prepend>
              <span class="text-body-small text-on-surface-variant">1</span>
            </template>
            <template #append>
              <span class="text-body-small text-on-surface-variant">64</span>
            </template>
          </v-slider>
        </div>

        <!-- 下载源 -->
        <div>
          <div class="text-body-medium mb-3">下载源</div>
          <v-btn-toggle
            v-model="settingsStore.downloadMirror"
            mandatory
            border
            divided
            rounded="pill"
            density="comfortable"
            variant="text"
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
          <p class="text-body-small text-on-surface-variant mt-2 mb-0">BMCL 镜像通常在国内访问更快</p>
        </div>
      </v-card-text>
    </v-card>

    <!-- 游戏语言 -->
    <v-card variant="flat" color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-4">
          <v-icon size="20" class="mr-2" color="on-surface-variant">mdi-translate</v-icon>
          <span class="text-title-medium">游戏语言</span>
        </div>
        <v-select
          density="comfortable"
          v-model="gameLanguage"
          :items="gameLanguages"
          item-title="title"
          item-value="value"
          label="选择游戏内显示语言"
          hide-details
        />
        <div class="text-body-small text-on-surface-variant mt-2">启动游戏时将自动设置 options.txt 中的语言选项</div>
      </v-card-text>
    </v-card>

    <!-- 外观 -->
    <v-card variant="flat" color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-4">
          <v-icon size="20" class="mr-2" color="on-surface-variant">mdi-palette-outline</v-icon>
          <span class="text-title-medium">外观</span>
        </div>
        <v-select
          density="comfortable"
          v-model="themePalette"
          :items="themePalettes"
          item-title="title"
          item-value="value"
          label="主题色方案（MD3）"
          hide-details
        />
        <div class="text-body-small text-on-surface-variant mt-2">
          主题色方案会在浅色/深色模式之间保持一致的 MD3 色调风格
        </div>
      </v-card-text>
    </v-card>

    <!-- 关于 -->
    <v-card variant="flat" color="surface-container">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-4">
          <v-icon size="20" class="mr-2" color="on-surface-variant">mdi-information-outline</v-icon>
          <span class="text-title-medium">关于</span>
        </div>
        <div class="d-flex align-center justify-space-between mb-2">
          <div class="text-body-medium text-on-surface-variant">应用版本</div>
          <v-chip size="small" color="primary" variant="tonal">{{ appVersion }}</v-chip>
        </div>
        <div class="text-body-small text-on-surface-variant">
          快捷键：Ctrl+D 下载，Ctrl+N 新建实例，Ctrl+I 实例管理，Ctrl+, 设置
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
