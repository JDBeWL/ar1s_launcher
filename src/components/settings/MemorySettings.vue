<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue';
import { useSettingsStore } from '../../stores/settings';
import { configApi } from '../../services';
import { logError } from '../../utils/logger';

const settingsStore = useSettingsStore();
const memoryWarning = ref('');
const autoMemoryEnabled = ref(false);
const memoryEfficiency = ref('');
const isInitialized = ref(false);

const totalMemoryGB = computed(() => (settingsStore.totalMemoryMB / 1024).toFixed(1));
const maxMemoryGB = computed(() => (settingsStore.maxMemory / 1024).toFixed(1));
const memoryPercentage = computed(() =>
  settingsStore.totalMemoryMB > 0 ? Math.round((settingsStore.maxMemory / settingsStore.totalMemoryMB) * 100) : 0,
);

// 给系统留出 4GB，避免分配值可以拉到物理内存总量导致整机卡死
const maxAllocatableMB = computed(() => Math.max(512, settingsStore.totalMemoryMB - 4096));

function clampMaxMemory() {
  const raw = Number(settingsStore.maxMemory);
  const clamped = Number.isFinite(raw) ? Math.min(Math.max(Math.round(raw), 512), maxAllocatableMB.value) : 512;
  settingsStore.maxMemory = clamped;
  settingsStore.saveMaxMemory();
}

async function checkMemoryWarning() {
  try {
    const warning = await configApi.checkMemoryWarning(settingsStore.maxMemory);
    memoryWarning.value = warning || '';
  } catch (err) {
    logError('Failed to check memory warning', err, 'MemorySettings');
    memoryWarning.value = '';
  }
}

async function loadAutoMemoryConfig() {
  try {
    const config = await configApi.getAutoMemoryConfig();
    autoMemoryEnabled.value = config.enabled;
  } catch (err) {
    logError('Failed to load auto memory config', err, 'MemorySettings');
  }
}

async function toggleAutoMemory() {
  try {
    await configApi.setAutoMemoryEnabled(autoMemoryEnabled.value);
    if (autoMemoryEnabled.value) {
      await applyAutoMemory();
    }
  } catch (err) {
    logError('Failed to toggle auto memory', err, 'MemorySettings');
  }
}

async function applyAutoMemory() {
  try {
    const recommendedMemory = await configApi.autoSetMemory();
    if (recommendedMemory !== null && recommendedMemory !== undefined) {
      settingsStore.maxMemory = recommendedMemory;
      await settingsStore.saveMaxMemory();
      await analyzeMemoryEfficiency();
    }
  } catch (err) {
    logError('Failed to apply auto memory', err, 'MemorySettings');
  }
}

async function analyzeMemoryEfficiency() {
  try {
    memoryEfficiency.value = await configApi.analyzeMemoryEfficiency(settingsStore.maxMemory);
  } catch (err) {
    logError('Failed to analyze memory efficiency', err, 'MemorySettings');
    memoryEfficiency.value = '';
  }
}

function setPresetMemory(mb: number) {
  settingsStore.maxMemory = Math.min(mb, maxAllocatableMB.value);
  settingsStore.saveMaxMemory();
}

watch(
  () => settingsStore.maxMemory,
  async () => {
    await checkMemoryWarning();
    await analyzeMemoryEfficiency();
  },
);

watch(autoMemoryEnabled, async () => {
  if (!isInitialized.value) return;
  await toggleAutoMemory();
});

onMounted(async () => {
  await settingsStore.loadSystemMemory();
  await settingsStore.loadSettings();
  await loadAutoMemoryConfig();
  await analyzeMemoryEfficiency();
  isInitialized.value = true;
});
</script>

<template>
  <div class="settings-group">
    <v-card variant="flat" color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-4">
          <v-icon size="20" class="mr-2" color="on-surface-variant">mdi-chart-donut</v-icon>
          <span class="text-title-medium">内存概览</span>
        </div>

        <v-row align="center">
          <v-col cols="12" sm="4">
            <div class="memory-stat text-center pa-4">
              <div class="text-headline-medium font-weight-bold text-primary num-mono">{{ totalMemoryGB }}</div>
              <div class="text-body-medium text-on-surface-variant">系统总内存 (GB)</div>
            </div>
          </v-col>
          <v-col cols="12" sm="4">
            <div class="memory-stat text-center pa-4">
              <div class="text-headline-medium font-weight-bold text-secondary num-mono">{{ maxMemoryGB }}</div>
              <div class="text-body-medium text-on-surface-variant">分配给游戏 (GB)</div>
            </div>
          </v-col>
          <v-col cols="12" sm="4">
            <div class="memory-stat text-center pa-4">
              <div class="text-headline-medium font-weight-bold text-info num-mono">{{ memoryPercentage }}%</div>
              <div class="text-body-medium text-on-surface-variant">占用比例</div>
            </div>
          </v-col>
        </v-row>
      </v-card-text>
    </v-card>

    <v-card variant="flat" color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center justify-space-between mb-1">
          <div class="d-flex align-center">
            <v-icon size="20" class="mr-2" color="on-surface-variant">mdi-auto-fix</v-icon>
            <span class="text-title-medium">自动内存管理</span>
          </div>
          <v-switch
            v-model="autoMemoryEnabled"
            aria-label="自动内存管理"
            hide-details
            inset
            density="compact"
            color="primary"
          />
        </div>
        <p class="text-body-medium text-on-surface-variant mb-0">根据系统可用内存自动设置最佳值，上限 8.5GB</p>

        <v-expand-transition>
          <div v-if="autoMemoryEnabled" class="mt-4">
            <v-alert color="primary-container" density="compact">
              <div class="d-flex align-center justify-space-between num-mono">
                <span class="text-on-primary-container">当前自动分配: {{ settingsStore.maxMemory }} MB</span>
                <v-btn size="small" variant="text" color="primary" @click="applyAutoMemory"> 重新计算 </v-btn>
              </div>
            </v-alert>
          </div>
        </v-expand-transition>
      </v-card-text>
    </v-card>

    <v-card variant="flat" v-if="!autoMemoryEnabled" color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-4">
          <v-icon size="20" class="mr-2" color="on-surface-variant">mdi-tune-vertical</v-icon>
          <span class="text-title-medium">手动设置</span>
        </div>

        <div class="mb-5">
          <div class="text-body-medium text-on-surface-variant mb-3">快捷预设</div>
          <div class="d-flex flex-wrap ga-2">
            <v-chip
              v-for="preset in [2048, 4096, 6144, 8192]"
              :key="preset"
              :variant="settingsStore.maxMemory === preset ? 'flat' : 'tonal'"
              :color="settingsStore.maxMemory === preset ? 'primary' : 'secondary'"
              :disabled="preset > maxAllocatableMB"
              @click="setPresetMemory(preset)"
            >
              {{ preset / 1024 }} GB
            </v-chip>
          </div>
        </div>

        <div class="mb-4">
          <div class="d-flex align-center justify-space-between mb-2">
            <span class="text-body-medium">自定义内存</span>
            <v-text-field
              v-model.number="settingsStore.maxMemory"
              type="number"
              density="compact"
              aria-label="自定义内存大小（MB）"
              suffix="MB"
              hide-details
              hide-spin-buttons
              style="max-width: 140px"
              @change="clampMaxMemory"
            />
          </div>
          <v-slider
            v-model="settingsStore.maxMemory"
            :min="512"
            :max="maxAllocatableMB"
            :step="128"
            hide-details
            thumb-label
            color="primary"
            @end="settingsStore.saveMaxMemory"
          >
            <template #prepend>
              <span class="text-body-small text-on-surface-variant num-mono">512 MB</span>
            </template>
            <template #append>
              <span class="text-body-small text-on-surface-variant num-mono">{{ maxAllocatableMB }} MB</span>
            </template>
          </v-slider>
        </div>
      </v-card-text>
    </v-card>

    <div v-if="memoryWarning || memoryEfficiency" class="mb-4">
      <v-alert v-if="memoryWarning" color="warning-container" density="compact" class="mb-2">
        <template #prepend>
          <v-icon color="on-warning-container">mdi-alert-outline</v-icon>
        </template>
        <span class="text-on-warning-container">{{ memoryWarning }}</span>
      </v-alert>

      <v-alert v-if="memoryEfficiency" color="secondary-container" density="compact">
        <template #prepend>
          <v-icon color="on-secondary-container">mdi-lightbulb-outline</v-icon>
        </template>
        <span class="text-on-secondary-container">{{ memoryEfficiency }}</span>
      </v-alert>
    </div>

    <v-alert color="secondary-container" class="tip-alert">
      <template #prepend>
        <v-icon size="20" color="on-secondary-container">mdi-information-outline</v-icon>
      </template>
      <template #title>
        <span class="text-body-medium font-weight-medium text-on-secondary-container">内存分配建议</span>
      </template>
      <ul class="text-body-small pl-4 mb-0 mt-1 text-on-secondary-container">
        <li>原版游戏: 2-4 GB</li>
        <li>轻量整合包: 4-6 GB</li>
        <li>大型整合包: 6-8 GB</li>
        <li>建议保留至少 2GB 给系统使用</li>
      </ul>
    </v-alert>
  </div>
</template>

<style scoped>
.memory-stat {
  background: rgb(var(--v-theme-surface-container-high));
  border-radius: 16px;
}
</style>
