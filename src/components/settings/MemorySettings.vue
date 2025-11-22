<script setup lang="ts">
import { ref, onMounted, computed, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore } from '../../stores/settings';

const settingsStore = useSettingsStore();
const memoryWarning = ref('');
const autoMemoryEnabled = ref(false);
const memoryEfficiency = ref('');

const totalMemoryGB = computed(() => (settingsStore.totalMemoryMB / 1024).toFixed(1));

// 检查内存设置是否超过90%并显示警告
async function checkMemoryWarning() {
  try {
    const warning = await invoke<string | null>('check_memory_warning', { memoryMb: settingsStore.maxMemory });
    memoryWarning.value = warning || '';
  } catch (err) {
    console.error('Failed to check memory warning:', err);
    memoryWarning.value = '';
  }
}

// 加载自动内存设置状态
async function loadAutoMemoryConfig() {
  try {
    const config = await invoke<{ enabled: boolean }>('get_auto_memory_config');
    autoMemoryEnabled.value = config.enabled;
  } catch (err) {
    console.error('Failed to load auto memory config:', err);
  }
}

// 切换自动内存设置
async function toggleAutoMemory() {
  try {
    await invoke('set_auto_memory_enabled', { enabled: autoMemoryEnabled.value });
    
    // 如果启用自动设置，立即应用推荐的内存
    if (autoMemoryEnabled.value) {
      await applyAutoMemory();
    }
  } catch (err) {
    console.error('Failed to toggle auto memory:', err);
  }
}

// 应用自动内存推荐
async function applyAutoMemory() {
  try {
    const recommendedMemory = await invoke<number | null>('auto_set_memory');
    if (recommendedMemory !== null && recommendedMemory !== undefined) {
      settingsStore.maxMemory = recommendedMemory;
      await settingsStore.saveMaxMemory();
      
      // 更新内存效率分析
      await analyzeMemoryEfficiency();
    }
  } catch (err) {
    console.error('Failed to apply auto memory:', err);
  }
}

// 分析内存使用效率
async function analyzeMemoryEfficiency() {
  try {
    const efficiency = await invoke<string>('analyze_memory_efficiency', { memoryMb: settingsStore.maxMemory });
    memoryEfficiency.value = efficiency;
  } catch (err) {
    console.error('Failed to analyze memory efficiency:', err);
    memoryEfficiency.value = '';
  }
}

// 监听内存设置变化，检查是否超过90%
watch(() => settingsStore.maxMemory, async () => {
  await checkMemoryWarning();
  await analyzeMemoryEfficiency();
});

// 监听自动内存设置变化
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
  <v-card>
    <v-card-title class="d-flex align-center">
      <v-icon class="mr-2">mdi-memory</v-icon>
      内存设置
    </v-card-title>
    <v-card-text class="pa-4">
      <!-- 系统内存信息 -->
      <div class="d-flex align-center justify-space-between mb-6">
        <div>
          <div class="text-subtitle-2">系统总内存</div>
          <div class="text-h6 text-primary">{{ settingsStore.totalMemoryMB }} MB (约 {{ totalMemoryGB }} GB)</div>
        </div>
        <v-chip color="primary" variant="outlined">
          <v-icon start>mdi-information</v-icon>
          可用内存
        </v-chip>
      </div>

      <!-- 自动内存设置 -->
      <v-switch
        v-model="autoMemoryEnabled"
        label="自动设置内存"
        color="primary"
        class="mb-6"
        hide-details
        hint="根据系统可用内存自动设置最佳内存大小，不超过8500MB"
        persistent-hint
      ></v-switch>

      <!-- 手动内存设置 -->
      <div v-if="!autoMemoryEnabled" class="mb-6">
        <div class="text-subtitle-2 mb-3">手动设置内存</div>
        <v-row align="center">
          <v-col cols="12" sm="8">
            <v-slider
              v-model="settingsStore.maxMemory"
              label="最大内存 (MB)"
              :min="512"
              :max="settingsStore.totalMemoryMB"
              :step="128"
              thumb-label
              :hint="`可用范围: 512MB - ${settingsStore.totalMemoryMB}MB`"
              persistent-hint
              @end="settingsStore.saveMaxMemory"
              hide-details
            ></v-slider>
          </v-col>
          <v-col cols="12" sm="4">
            <v-text-field
              v-model.number="settingsStore.maxMemory"
              type="number"
              label="内存大小"
              suffix="MB"
              :rules="[
                v => !!v || '必须输入内存大小',
                v => (v >= 512 && v <= settingsStore.totalMemoryMB) || `必须在512-${settingsStore.totalMemoryMB}MB之间`
              ]"
              hide-spin-buttons
              @change="settingsStore.saveMaxMemory"
              hide-details
            ></v-text-field>
          </v-col>
        </v-row>
      </div>

      <!-- 自动内存状态 -->
      <div v-if="autoMemoryEnabled" class="mb-6">
        <v-alert
          type="info"
          variant="tonal"
          density="compact"
          hide-details
        >
          <div class="d-flex align-center">
            <v-icon class="mr-2">mdi-auto-fix</v-icon>
            <span>自动内存设置已启用：当前内存 {{ settingsStore.maxMemory }}MB</span>
            <v-btn
              size="small"
              variant="text"
              class="ml-auto"
              @click="applyAutoMemory"
            >
              重新计算
            </v-btn>
          </div>
        </v-alert>
      </div>

      <!-- 内存分析信息 -->
      <div v-if="memoryEfficiency || memoryWarning" class="mb-2">
        <v-alert
          v-if="memoryEfficiency"
          type="info"
          variant="tonal"
          density="compact"
          class="mb-2"
          hide-details
        >
          <v-icon class="mr-2">mdi-chart-line</v-icon>
          {{ memoryEfficiency }}
        </v-alert>
        
        <v-alert
          v-if="memoryWarning"
          type="warning"
          variant="tonal"
          density="compact"
          hide-details
        >
          <v-icon class="mr-2">mdi-alert</v-icon>
          {{ memoryWarning }}
        </v-alert>
      </div>
    </v-card-text>
  </v-card>
</template>
