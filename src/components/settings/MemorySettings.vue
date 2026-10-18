<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch } from 'vue';
import { useSettingsStore } from '../../stores/settings';
import { configApi } from '../../services';
import { logError } from '../../utils/logger';

const settingsStore = useSettingsStore();
const memoryWarning = ref('');

const autoMemoryEnabled = computed(() => settingsStore.autoMemoryEnabled);
const totalMemoryGB = computed(() => (settingsStore.totalMemoryMB / 1024).toFixed(1));
const usedMemoryGB = computed(() => (settingsStore.usedMemoryMB / 1024).toFixed(1));
const availableMemoryGB = computed(() => (settingsStore.availableMemoryMB / 1024).toFixed(1));
const maxMemoryGB = computed(() => (settingsStore.maxMemory / 1024).toFixed(1));
const memoryPercentage = computed(() =>
  settingsStore.totalMemoryMB > 0 ? Math.round((settingsStore.maxMemory / settingsStore.totalMemoryMB) * 100) : 0,
);
const projectedUsedMB = computed(() => settingsStore.usedMemoryMB + settingsStore.maxMemory);
const projectedUsedGB = computed(() => (projectedUsedMB.value / 1024).toFixed(1));
const projectedPercent = computed(() =>
  settingsStore.totalMemoryMB > 0 ? Math.round((projectedUsedMB.value / settingsStore.totalMemoryMB) * 100) : 0,
);
const projectedBarPercent = computed(() => Math.min(100, Math.max(0, projectedPercent.value)));
const projectedColor = computed(() => {
  if (projectedPercent.value > 100) return 'error';
  if (projectedPercent.value >= 90) return 'warning';
  return 'primary';
});
const projectedWarning = computed(() => {
  if (settingsStore.totalMemoryMB <= 0) return '';
  if (projectedPercent.value > 100) return '预计占用会超过物理内存，可能导致系统卡顿或游戏崩溃';
  if (projectedPercent.value >= 90) return '启动后预计占用接近系统上限，建议降低分配或关闭其他程序';
  return '';
});

// 手动模式上限就是物理内存总量，分配过高的风险由「预计占用」提示承担
const maxAllocatableMB = computed(() => Math.max(512, settingsStore.totalMemoryMB));

function clampMaxMemory() {
  const raw = Number(settingsStore.maxMemory);
  const clamped = Number.isFinite(raw) ? Math.min(Math.max(Math.round(raw), 512), maxAllocatableMB.value) : 512;
  settingsStore.maxMemory = clamped;
  settingsStore.saveMaxMemory();
}

async function checkMemoryWarning() {
  if (settingsStore.autoMemoryEnabled) {
    memoryWarning.value = '';
    return;
  }
  try {
    const warning = await configApi.checkMemoryWarning(settingsStore.maxMemory);
    memoryWarning.value = warning || '';
  } catch (err) {
    logError('Failed to check memory warning', err, 'MemorySettings');
    memoryWarning.value = '';
  }
}

async function toggleAutoMemory(value: boolean | null) {
  settingsStore.autoMemoryEnabled = value === true;
  await settingsStore.saveAutoMemoryEnabled();
  if (settingsStore.autoMemoryEnabled) {
    memoryWarning.value = '';
  } else {
    await checkMemoryWarning();
  }
}

function setPresetMemory(mb: number) {
  settingsStore.maxMemory = Math.min(mb, maxAllocatableMB.value);
  settingsStore.saveMaxMemory();
}

watch(
  () => settingsStore.maxMemory,
  () => {
    if (!settingsStore.autoMemoryEnabled) void checkMemoryWarning();
  },
);

let refreshTimer: ReturnType<typeof setInterval> | undefined;

onMounted(async () => {
  await settingsStore.loadSystemMemory();
  await settingsStore.loadSettings();
  await checkMemoryWarning();
  // 手动分配时占用会变，页面停留期间每 5 秒刷新一次实际占用
  refreshTimer = setInterval(() => void settingsStore.loadSystemMemory(), 5000);
});

onUnmounted(() => {
  if (refreshTimer !== undefined) clearInterval(refreshTimer);
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
              <div class="text-headline-medium font-weight-bold text-secondary num-mono">
                {{ autoMemoryEnabled ? '自动' : maxMemoryGB }}
              </div>
              <div class="text-body-medium text-on-surface-variant">
                {{ autoMemoryEnabled ? '启动时按实例计算' : '分配给游戏 (GB)' }}
              </div>
            </div>
          </v-col>
          <v-col cols="12" sm="4">
            <div class="memory-stat text-center pa-4">
              <div class="text-headline-medium font-weight-bold text-info num-mono">
                {{ autoMemoryEnabled ? '—' : memoryPercentage + '%' }}
              </div>
              <div class="text-body-medium text-on-surface-variant">
                {{ autoMemoryEnabled ? '启动时计算' : '占用比例' }}
              </div>
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
            :model-value="autoMemoryEnabled"
            aria-label="自动内存管理"
            hide-details
            inset
            density="compact"
            color="primary"
            @update:model-value="toggleAutoMemory"
          />
        </div>
        <p class="text-body-medium text-on-surface-variant mb-0">
          默认目标 4 GB，按实例规模抬高；最多使用当前可用内存的 75%，范围 2–16 GB
        </p>

        <v-expand-transition>
          <div v-if="autoMemoryEnabled" class="mt-4">
            <v-alert color="primary-container" density="compact">
              <span class="text-on-primary-container text-body-medium">
                启动时动态计算 -Xmx/-Xms：默认 4 GB，最少 2 GB、最多 16 GB，上限为当前可用内存的 75%；32 位 Java 硬上限
                1.5 GB。每张实例卡片会显示预计分配值，手动设置值关闭自动后继续使用。
              </span>
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

        <div class="memory-usage mb-5">
          <div class="d-flex justify-space-between align-center text-body-small mb-1">
            <span class="text-on-surface-variant">系统当前占用</span>
            <span class="num-mono">
              {{ usedMemoryGB }} / {{ totalMemoryGB }} GB（{{ Math.round(settingsStore.memoryUsagePercent) }}%）
            </span>
          </div>
          <div class="d-flex justify-space-between align-center text-body-small mb-1">
            <span class="text-on-surface-variant">当前可用内存</span>
            <span class="num-mono">{{ availableMemoryGB }} GB</span>
          </div>
          <div class="d-flex justify-space-between align-center text-body-small mb-1">
            <span class="text-on-surface-variant">游戏预计占用</span>
            <span class="num-mono">{{ maxMemoryGB }} GB</span>
          </div>
          <div class="d-flex justify-space-between align-center text-body-small">
            <span class="text-on-surface-variant">启动后预计系统占用</span>
            <span class="num-mono">{{ projectedUsedGB }} / {{ totalMemoryGB }} GB（{{ projectedPercent }}%）</span>
          </div>
          <v-progress-linear
            :model-value="projectedBarPercent"
            :color="projectedColor"
            bg-color="surface-container-highest"
            height="8"
            rounded
            class="mt-3"
          />
          <div v-if="projectedWarning" class="text-body-small text-warning mt-2">{{ projectedWarning }}</div>
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

    <v-alert v-if="memoryWarning" color="warning-container" density="compact" class="mb-4">
      <template #prepend>
        <v-icon color="on-warning-container">mdi-alert-outline</v-icon>
      </template>
      <span class="text-on-warning-container">{{ memoryWarning }}</span>
    </v-alert>

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
        <li>自动模式默认按 4 GB 目标分配，大型整合包会按需求抬高；最多使用当前可用内存的 75%（上限 16 GB）</li>
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
