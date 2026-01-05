<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";

interface WindowSettingsData {
  width: number | null;
  height: number | null;
  fullscreen: boolean;
}

const windowWidth = ref<number | null>(null);
const windowHeight = ref<number | null>(null);
const fullscreen = ref(false);
const useCustomSize = ref(false);

const presets = [
  { title: '默认 (游戏决定)', width: null, height: null },
  { title: '854 × 480', width: 854, height: 480 },
  { title: '1280 × 720 (720p)', width: 1280, height: 720 },
  { title: '1600 × 900', width: 1600, height: 900 },
  { title: '1920 × 1080 (1080p)', width: 1920, height: 1080 },
  { title: '2560 × 1440 (2K)', width: 2560, height: 1440 },
];

const currentPreset = computed(() => {
  if (!useCustomSize.value) {
    return presets.find(p => p.width === windowWidth.value && p.height === windowHeight.value) || presets[0];
  }
  return null;
});

async function loadSettings() {
  try {
    const settings = await invoke<WindowSettingsData>('get_window_settings');
    windowWidth.value = settings.width;
    windowHeight.value = settings.height;
    fullscreen.value = settings.fullscreen;
    useCustomSize.value = settings.width !== null && !presets.some(p => p.width === settings.width && p.height === settings.height);
  } catch (err) {
    console.error('Failed to load window settings:', err);
  }
}

async function saveSettings() {
  try {
    await invoke('set_window_settings', {
      width: windowWidth.value,
      height: windowHeight.value,
      fullscreen: fullscreen.value,
    });
  } catch (err) {
    console.error('Failed to save window settings:', err);
  }
}

function selectPreset(preset: typeof presets[0]) {
  windowWidth.value = preset.width;
  windowHeight.value = preset.height;
  useCustomSize.value = false;
  saveSettings();
}

function enableCustomSize() {
  useCustomSize.value = true;
  if (!windowWidth.value) windowWidth.value = 854;
  if (!windowHeight.value) windowHeight.value = 480;
}

function onCustomSizeChange() {
  saveSettings();
}

function toggleFullscreen() {
  saveSettings();
}

onMounted(() => {
  loadSettings();
});
</script>

<template>
  <div class="settings-group">
    <!-- 标题 -->
    <div class="group-header mb-5">
      <div class="d-flex align-center">
        <v-avatar size="48" color="primary-container" class="mr-3">
          <v-icon size="24" color="on-primary-container">mdi-monitor</v-icon>
        </v-avatar>
        <div>
          <h2 class="text-h6 font-weight-bold">窗口设置</h2>
          <p class="text-body-2 text-on-surface-variant mb-0">配置游戏启动时的窗口大小</p>
        </div>
      </div>
    </div>

    <!-- 全屏模式 -->
    <v-card color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center justify-space-between">
          <div class="d-flex align-center">
            <v-icon class="mr-2" color="on-surface-variant">mdi-fullscreen</v-icon>
            <div>
              <span class="text-subtitle-1 font-weight-medium">全屏模式</span>
              <p class="text-body-2 text-on-surface-variant mb-0">启动游戏时直接进入全屏</p>
            </div>
          </div>
          <v-switch
            v-model="fullscreen"
            hide-details
            density="compact"
            color="primary"
            @update:model-value="toggleFullscreen"
          />
        </div>
      </v-card-text>
    </v-card>

    <!-- 窗口大小 -->
    <v-card v-if="!fullscreen" color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-4">
          <v-icon class="mr-2" color="on-surface-variant">mdi-resize</v-icon>
          <span class="text-subtitle-1 font-weight-medium">窗口大小</span>
        </div>

        <!-- 预设选择 -->
        <div class="mb-4">
          <div class="text-body-2 text-on-surface-variant mb-3">快捷预设</div>
          <div class="d-flex flex-wrap ga-2">
            <v-chip
              v-for="preset in presets"
              :key="preset.title"
              :variant="!useCustomSize && currentPreset?.title === preset.title ? 'flat' : 'tonal'"
              :color="!useCustomSize && currentPreset?.title === preset.title ? 'primary' : 'secondary'"
              @click="selectPreset(preset)"
            >
              {{ preset.title }}
            </v-chip>
            <v-chip
              :variant="useCustomSize ? 'flat' : 'tonal'"
              :color="useCustomSize ? 'primary' : 'secondary'"
              @click="enableCustomSize"
            >
              自定义
            </v-chip>
          </div>
        </div>

        <!-- 自定义大小 -->
        <v-expand-transition>
          <div v-if="useCustomSize">
            <v-divider class="my-4" />
            <v-row dense>
              <v-col cols="6">
                <v-text-field
                  v-model.number="windowWidth"
                  type="number"
                  label="宽度"
                  suffix="px"
                  hide-details
                  :min="640"
                  :max="7680"
                  @change="onCustomSizeChange"
                >
                  <template #prepend-inner>
                    <v-icon size="20" color="on-surface-variant">mdi-arrow-left-right</v-icon>
                  </template>
                </v-text-field>
              </v-col>
              <v-col cols="6">
                <v-text-field
                  v-model.number="windowHeight"
                  type="number"
                  label="高度"
                  suffix="px"
                  hide-details
                  :min="480"
                  :max="4320"
                  @change="onCustomSizeChange"
                >
                  <template #prepend-inner>
                    <v-icon size="20" color="on-surface-variant">mdi-arrow-up-down</v-icon>
                  </template>
                </v-text-field>
              </v-col>
            </v-row>
          </div>
        </v-expand-transition>
      </v-card-text>
    </v-card>

    <!-- 提示 -->
    <v-alert color="tertiary-container">
      <template #prepend>
        <v-icon color="on-tertiary-container">mdi-information-outline</v-icon>
      </template>
      <template #title>
        <span class="text-body-2 font-weight-medium text-on-tertiary-container">关于窗口设置</span>
      </template>
      <ul class="text-body-2 pl-4 mb-0 mt-1 text-on-tertiary-container">
        <li>某些整合包的 KubeJS 脚本可能与特定窗口大小冲突</li>
        <li>如果游戏启动崩溃，尝试使用较小的窗口大小</li>
        <li>设置为"默认"将由游戏自行决定窗口大小</li>
      </ul>
    </v-alert>
  </div>
</template>

<style scoped>
.settings-group {
  margin-bottom: 32px;
}

.group-header {
  padding-bottom: 16px;
  border-bottom: 1px solid rgb(var(--v-theme-outline-variant));
}
</style>
