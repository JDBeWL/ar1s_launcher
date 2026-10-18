<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue';
import { open } from '@tauri-apps/plugin-dialog';
import { listen } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { useSettingsStore } from '../../stores/settings';
import { configApi } from '../../services';
import { logError } from '../../utils/logger';
import { getThemePalette } from '../../utils/storage';
import ThemePalettePicker from './ThemePalettePicker.vue';
import NetworkProxySettings from './NetworkProxySettings.vue';

const settingsStore = useSettingsStore();
const gameDir = ref('');
const versionIsolation = ref(true);
const downloadThreads = ref(16);
const downloadThreadsInput = ref('16');
const isInitialized = ref(false);
const appVersion = __APP_VERSION__;
const themePalette = ref(getThemePalette());

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
    downloadThreads.value = clampDownloadThreads(await configApi.getDownloadThreads());
    downloadThreadsInput.value = String(downloadThreads.value);
  } catch (err) {
    logError('Failed to get download threads', err, 'GeneralSettings');
  }
}

function clampDownloadThreads(value: number): number {
  return Math.min(256, Math.max(1, Math.round(value)));
}

async function saveDownloadThreads() {
  try {
    await configApi.setDownloadThreads(downloadThreads.value);
  } catch (err) {
    logError('Failed to set download threads', err, 'GeneralSettings');
  }
}

function commitDownloadThreads() {
  const raw = downloadThreadsInput.value.trim();
  const parsed = raw === '' ? Number.NaN : Number(raw);
  if (!Number.isFinite(parsed)) {
    downloadThreadsInput.value = String(downloadThreads.value);
    return;
  }

  const clamped = clampDownloadThreads(parsed);
  downloadThreads.value = clamped;
  downloadThreadsInput.value = String(clamped);
  void saveDownloadThreads();
}

async function loadVersionIsolation() {
  try {
    const isolation = await configApi.loadConfigKey('versionIsolation');
    if (isolation !== null) {
      versionIsolation.value = isolation === 'true';
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

// onMounted 的 load 会给这些 ref 赋值，没有守卫就会一进页面把原值回写一遍
watch(versionIsolation, (v) => {
  if (!isInitialized.value) return;
  saveIsolationSetting('versionIsolation', v);
});
watch(gameLanguage, async (v) => {
  if (!isInitialized.value) return;
  try {
    await configApi.saveConfigKey('language', v);
  } catch (err) {
    logError('Failed to save language', err, 'GeneralSettings');
  }
});

watch(
  () => settingsStore.downloadMirror,
  async () => {
    if (!isInitialized.value) return;
    await settingsStore.saveDownloadMirror();
  },
);

watch(downloadThreads, (value) => {
  downloadThreadsInput.value = String(value);
});

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

  isInitialized.value = true;

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
        <p class="text-body-medium text-on-surface-variant mb-0">
          开启后每个版本使用独立目录，存档、模组、配置互不影响；关闭后所有版本共用根目录的 mods、存档与配置。
        </p>

        <v-alert v-if="!versionIsolation" type="warning" variant="tonal" density="compact" class="mt-3 text-body-small">
          关闭隔离后游戏读取的是根目录，启动器会把模组装到根目录的 mods；实例目录里已有的模组不会被加载。
        </v-alert>
      </v-card-text>
    </v-card>

    <v-card variant="flat" color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-4">
          <v-icon size="20" class="mr-2" color="on-surface-variant">mdi-download-outline</v-icon>
          <span class="text-title-medium">下载设置</span>
        </div>

        <div class="mb-5">
          <div class="d-flex align-center justify-space-between mb-2">
            <span class="text-body-medium">下载线程数</span>
            <v-text-field
              v-model="downloadThreadsInput"
              type="number"
              density="compact"
              aria-label="下载线程数"
              suffix="线程"
              hide-details
              hide-spin-buttons
              class="download-threads-input"
              @blur="commitDownloadThreads"
              @keyup.enter="commitDownloadThreads"
            />
          </div>
          <v-slider
            v-model="downloadThreads"
            :min="1"
            :max="256"
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
              <span class="text-body-small text-on-surface-variant">256</span>
            </template>
          </v-slider>
          <p class="text-body-small text-on-surface-variant mt-2 mb-0">范围 1–256，过高可能触发下载源限速</p>
        </div>

        <div>
          <div class="text-body-medium mb-3">优先下载源</div>
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
          <p class="text-body-small text-on-surface-variant mt-2 mb-0">
            优先使用所选源；BMCL 镜像不可用时自动回退官方源
          </p>
        </div>
      </v-card-text>
    </v-card>

    <NetworkProxySettings />

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

    <v-card variant="flat" color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-4">
          <v-icon size="20" class="mr-2" color="on-surface-variant">mdi-palette-outline</v-icon>
          <span class="text-title-medium">外观</span>
        </div>
        <ThemePalettePicker v-model="themePalette" />
        <div class="text-body-small text-on-surface-variant mt-3">
          配色由种子色推导，浅色与深色模式共用同一套色调关系
        </div>
      </v-card-text>
    </v-card>

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
.download-threads-input {
  max-width: 140px;
}
</style>
