<script setup lang="ts">
import { ref, onMounted, computed, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore } from '../../stores/settings';

const settingsStore = useSettingsStore();
const memoryWarning = ref('');
const autoMemoryEnabled = ref(false);
const memoryEfficiency = ref('');

const totalMemoryGB = computed(() => (settingsStore.totalMemoryMB / 1024).toFixed(1));
const maxMemoryGB = computed(() => (settingsStore.maxMemory / 1024).toFixed(1));
const memoryPercentage = computed(() => 
  Math.round((settingsStore.maxMemory / settingsStore.totalMemoryMB) * 100)
);

async function checkMemoryWarning() {
  try {
    const warning = await invoke<string | null>('check_memory_warning', { memoryMb: settingsStore.maxMemory });
    memoryWarning.value = warning || '';
  } catch (err) {
    console.error('Failed to check memory warning:', err);
    memoryWarning.value = '';
  }
}

async function loadAutoMemoryConfig() {
  try {
    const config = await invoke<{ enabled: boolean }>('get_auto_memory_config');
    autoMemoryEnabled.value = config.enabled;
  } catch (err) {
    console.error('Failed to load auto memory config:', err);
  }
}

async function toggleAutoMemory() {
  try {
    await invoke('set_auto_memory_enabled', { enabled: autoMemoryEnabled.value });
    if (autoMemoryEnabled.value) {
      await applyAutoMemory();
    }
  } catch (err) {
    console.error('Failed to toggle auto memory:', err);
  }
}

async function applyAutoMemory() {
  try {
    const recommendedMemory = await invoke<number | null>('auto_set_memory');
    if (recommendedMemory !== null && recommendedMemory !== undefined) {
      settingsStore.maxMemory = recommendedMemory;
      await settingsStore.saveMaxMemory();
      await analyzeMemoryEfficiency();
    }
  } catch (err) {
    console.error('Failed to apply auto memory:', err);
  }
}

async function analyzeMemoryEfficiency() {
  try {
    const efficiency = await invoke<string>('analyze_memory_efficiency', { memoryMb: settingsStore.maxMemory });
    memoryEfficiency.value = efficiency;
  } catch (err) {
    console.error('Failed to analyze memory efficiency:', err);
    memoryEfficiency.value = '';
  }
}

function setPresetMemory(mb: number) {
  settingsStore.maxMemory = Math.min(mb, settingsStore.totalMemoryMB);
  settingsStore.saveMaxMemory();
}

watch(() => settingsStore.maxMemory, async () => {
  await checkMemoryWarning();
  await analyzeMemoryEfficiency();
});

watch(autoMemoryEnabled, async () => {
  await toggleAutoMemory();
});

onMounted(async () => {
  await settingsStore.loadSystemMemory();
  await settingsStore.loadMaxMemory();
  await loadAutoMemoryConfig();
  await analyzeMemoryEfficiency();
});
</script>

<template>
  <div class="settings-group">
    <!-- 标题 -->
    <div class="group-header mb-5">
      <div class="d-flex align-center">
        <v-avatar size="48" color="primary-container" class="mr-3">
          <v-icon size="24" color="on-primary-container">mdi-memory</v-icon>
        </v-avatar>
        <div>
          <h2 class="text-h6 font-weight-bold">内存管理</h2>
          <p class="text-body-2 text-on-surface-variant mb-0">配置游戏可用的内存大小</p>
        </div>
      </div>
    </div>

    <!-- 系统内存概览 -->
    <v-card color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-4">
          <v-icon class="mr-2" color="on-surface-variant">mdi-chart-donut</v-icon>
          <span class="text-subtitle-1 font-weight-medium">内存概览</span>
        </div>

        <v-row align="center">
          <v-col cols="12" sm="4">
            <div class="memory-stat text-center pa-4">
              <div class="text-h4 font-weight-bold text-primary">{{ totalMemoryGB }}</div>
              <div class="text-body-2 text-on-surface-variant">系统总内存 (GB)</div>
            </div>
          </v-col>
          <v-col cols="12" sm="4">
            <div class="memory-stat text-center pa-4">
              <div class="text-h4 font-weight-bold text-secondary">{{ maxMemoryGB }}</div>
              <div class="text-body-2 text-on-surface-variant">分配给游戏 (GB)</div>
            </div>
          </v-col>
          <v-col cols="12" sm="4">
            <div class="memory-stat text-center pa-4">
              <div class="text-h4 font-weight-bold text-tertiary">{{ memoryPercentage }}%</div>
              <div class="text-body-2 text-on-surface-variant">占用比例</div>
            </div>
          </v-col>
        </v-row>
      </v-card-text>
    </v-card>

    <!-- 自动内存设置 -->
    <v-card color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center justify-space-between mb-1">
          <div class="d-flex align-center">
            <v-icon class="mr-2" color="on-surface-variant">mdi-auto-fix</v-icon>
            <span class="text-subtitle-1 font-weight-medium">自动内存管理</span>
          </div>
          <v-switch
            v-model="autoMemoryEnabled"
            hide-details
            density="compact"
            color="primary"
          />
        </div>
        <p class="text-body-2 text-on-surface-variant mb-0">
          根据系统可用内存自动设置最佳值，上限 8.5GB
        </p>

        <v-expand-transition>
          <div v-if="autoMemoryEnabled" class="mt-4">
            <v-alert color="primary-container" density="compact">
              <div class="d-flex align-center justify-space-between">
                <span class="text-on-primary-container">当前自动分配: {{ settingsStore.maxMemory }} MB</span>
                <v-btn
                  size="small"
                  variant="text"
                  color="primary"
                  @click="applyAutoMemory"
                >
                  重新计算
                </v-btn>
              </div>
            </v-alert>
          </div>
        </v-expand-transition>
      </v-card-text>
    </v-card>

    <!-- 手动内存设置 -->
    <v-card v-if="!autoMemoryEnabled" color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-4">
          <v-icon class="mr-2" color="on-surface-variant">mdi-tune-vertical</v-icon>
          <span class="text-subtitle-1 font-weight-medium">手动设置</span>
        </div>

        <!-- 快捷预设 -->
        <div class="mb-5">
          <div class="text-body-2 text-on-surface-variant mb-3">快捷预设</div>
          <div class="d-flex flex-wrap ga-2">
            <v-chip
              v-for="preset in [2048, 4096, 6144, 8192]"
              :key="preset"
              :variant="settingsStore.maxMemory === preset ? 'flat' : 'tonal'"
              :color="settingsStore.maxMemory === preset ? 'primary' : 'secondary'"
              :disabled="preset > settingsStore.totalMemoryMB"
              @click="setPresetMemory(preset)"
            >
              {{ preset / 1024 }} GB
            </v-chip>
          </div>
        </div>

        <!-- 滑块 -->
        <div class="mb-4">
          <div class="d-flex align-center justify-space-between mb-2">
            <span class="text-body-2">自定义内存</span>
            <v-text-field
              v-model.number="settingsStore.maxMemory"
              type="number"
              density="compact"
              suffix="MB"
              hide-details
              hide-spin-buttons
              style="max-width: 140px"
              @change="settingsStore.saveMaxMemory"
            />
          </div>
          <v-slider
            v-model="settingsStore.maxMemory"
            :min="512"
            :max="settingsStore.totalMemoryMB"
            :step="128"
            hide-details
            color="primary"
            @end="settingsStore.saveMaxMemory"
          >
            <template #prepend>
              <span class="text-caption text-on-surface-variant">512 MB</span>
            </template>
            <template #append>
              <span class="text-caption text-on-surface-variant">{{ settingsStore.totalMemoryMB }} MB</span>
            </template>
          </v-slider>
        </div>

        <!-- 进度条 -->
        <v-progress-linear
          :model-value="memoryPercentage"
          height="8"
          rounded
          color="primary"
        />
      </v-card-text>
    </v-card>

    <!-- 警告和建议 -->
    <div v-if="memoryWarning || memoryEfficiency" class="mb-4">
      <v-alert
        v-if="memoryWarning"
        color="warning-container"
        density="compact"
        class="mb-2"
      >
        <template #prepend>
          <v-icon color="on-warning-container">mdi-alert-outline</v-icon>
        </template>
        <span class="text-on-warning-container">{{ memoryWarning }}</span>
      </v-alert>
      
      <v-alert
        v-if="memoryEfficiency"
        color="secondary-container"
        density="compact"
      >
        <template #prepend>
          <v-icon color="on-secondary-container">mdi-lightbulb-outline</v-icon>
        </template>
        <span class="text-on-secondary-container">{{ memoryEfficiency }}</span>
      </v-alert>
    </div>

    <!-- 内存建议 -->
    <v-alert color="tertiary-container">
      <template #prepend>
        <v-icon color="on-tertiary-container">mdi-information-outline</v-icon>
      </template>
      <template #title>
        <span class="text-body-2 font-weight-medium text-on-tertiary-container">内存分配建议</span>
      </template>
      <ul class="text-body-2 pl-4 mb-0 mt-1 text-on-tertiary-container">
        <li>原版游戏: 2-4 GB</li>
        <li>轻量整合包: 4-6 GB</li>
        <li>大型整合包: 6-8 GB</li>
        <li>建议保留至少 2GB 给系统使用</li>
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

.memory-stat {
  background: rgb(var(--v-theme-surface-container-high));
  border-radius: 16px;
}
</style>
