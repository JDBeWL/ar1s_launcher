<script setup lang="ts">
import { ref, onMounted, computed, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from '@tauri-apps/api/event';
import { useSettingsStore } from '../stores/settings';

function formatJavaPath(rawPath: string) {
  if (!rawPath) return '';
  // 统一转换为正斜杠显示
  return rawPath.replace(/\\/g, '/');
}

const settingsStore = useSettingsStore()
const gameDir = ref('')
const versionIsolation = ref(true)
const javaPath = ref('')
const isJavaPathValid = ref(false)
const loadingJava = ref(false)
const downloadThreads = ref(8);
const memoryWarning = ref('');
const autoMemoryEnabled = ref(false);
const memoryEfficiency = ref('');

const totalMemoryGB = computed(() => (settingsStore.totalMemoryMB / 1024).toFixed(1));

// 内存相关函数已迁移到Pinia store

// 加载已保存的游戏目录
async function loadGameDir() {
  try {
    const dir = await invoke('get_game_dir');
    gameDir.value = dir as string;
  } catch (err) {
    console.error('Failed to get game directory:', err);
  }
}

// 加载已保存的Java路径
async function loadJavaPath() {
  try {
    javaPath.value = (await invoke('load_config_key', { key: 'javaPath' })) as string;
    isJavaPathValid.value = await invoke('validate_java_path', { path: javaPath.value });
  } catch (error) {
    console.error('Failed to load Java path:', error);
  }
}

// 查找系统中的Java安装
async function findJavaInstallations() {
  try {
    loadingJava.value = true;
    await settingsStore.findJavaInstallations();
    
    // 如果找到了Java安装但还没有设置Java路径，则自动选择第一个
    if (settingsStore.javaInstallations.length > 0 && !javaPath.value) {
      javaPath.value = settingsStore.javaInstallations[0];
      await setJavaPath(javaPath.value);
    }
    
    loadingJava.value = false;
  } catch (err) {
    console.error('Failed to find Java installations:', err);
    loadingJava.value = false;
  }
}

// 设置Java路径
async function setJavaPath(path: string) {
  try {
    await invoke('save_config_key', { key: 'javaPath', value: path });
  } catch (err) {
    console.error('Failed to set Java path:', err);
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

// 获取和保存下载线程数
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

// 加载和保存版本隔离设置
async function loadVersionIsolation() {
  try {
    const isolation = await invoke('load_config_key', { key: 'versionIsolation' });
    versionIsolation.value = isolation === 'true';
  } catch (err) {
    console.error('Failed to load version isolation:', err);
  }
}

watch(versionIsolation, async (newValue) => {
  try {
    await invoke('save_config_key', { key: 'versionIsolation', value: newValue.toString() });
  } catch (err) {
    console.error('Failed to save version isolation:', err);
  }
});

watch(() => settingsStore.downloadMirror, async () => {
  await settingsStore.saveDownloadMirror();
});

// 监听内存设置变化，检查是否超过90%
watch(() => settingsStore.maxMemory, async () => {
  await checkMemoryWarning();
  await analyzeMemoryEfficiency();
});

// 监听自动内存设置变化
watch(autoMemoryEnabled, async () => {
  await toggleAutoMemory();
});

// 在组件挂载时加载所有设置
onMounted(async () => {
  await settingsStore.loadSystemMemory();
  await settingsStore.loadMaxMemory();
  await settingsStore.loadDownloadMirror();
  await loadGameDir();
  await loadJavaPath();
  await loadDownloadThreads();
  await loadVersionIsolation();
  await loadAutoMemoryConfig();
  await analyzeMemoryEfficiency();
  
  // 只在启动时查找一次Java安装，之后保持状态
  if (!settingsStore.hasFoundJavaInstallations && settingsStore.javaInstallations.length === 0) {
    await findJavaInstallations();
  }
  
  // 监听游戏目录变更事件
  await listen('game-dir-changed', (event) => {
    gameDir.value = event.payload as string;
  });
});
</script>

<template>
  <v-container class="pa-4">
    <v-row>
      <!-- 游戏设置 -->
      <v-col cols="12" md="6">
        <v-card class="h-100">
          <v-card-title class="d-flex align-center">
            <v-icon class="mr-2">mdi-gamepad-variant</v-icon>
            游戏设置
          </v-card-title>
          <v-card-text class="pa-4">
            <v-text-field
              v-model="gameDir"
              label="游戏目录"
              append-inner-icon="mdi-folder"
              @click:append-inner="selectGameDir"
              readonly
              hide-details
              class="mb-6"
            ></v-text-field>

            <v-switch
              v-model="versionIsolation"
              label="版本隔离"
              color="primary"
              hide-details
              hint="为每个版本创建独立的文件夹结构"
              persistent-hint
            ></v-switch>
          </v-card-text>
        </v-card>
      </v-col>

      <!-- 下载设置 -->
      <v-col cols="12" md="6">
        <v-card class="h-100">
          <v-card-title class="d-flex align-center">
            <v-icon class="mr-2">mdi-download</v-icon>
            下载设置
          </v-card-title>
          <v-card-text class="pa-4">
            <v-slider
              v-model="downloadThreads"
              label="下载线程数"
              :min="1"
              :max="16"
              :step="1"
              thumb-label
              show-ticks="always"
              persistent-hint
              hint="设置多线程下载时使用的线程数量"
              @end="saveDownloadThreads"
              hide-details
              class="mb-6"
            ></v-slider>

            <div>
              <div class="text-subtitle-2 font-weight-medium mb-3">下载源</div>
              <v-radio-group v-model="settingsStore.downloadMirror" inline hide-details>
                <v-radio label="官方源" value="official"></v-radio>
                <v-radio label="BMCL 镜像" value="bmcl"></v-radio>
              </v-radio-group>
            </div>
          </v-card-text>
        </v-card>
      </v-col>
    </v-row>

    <!-- 内存设置 -->
    <v-row class="mt-4">
      <v-col cols="12">
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
      </v-col>
    </v-row>

    <!-- Java设置 -->
    <v-row class="mt-4">
      <v-col cols="12">
        <v-card>
          <v-card-title class="d-flex align-center">
            <v-icon class="mr-2">mdi-language-java</v-icon>
            Java 设置
          </v-card-title>
          <v-card-text class="pa-4">
            <v-combobox
              v-model="javaPath"
              :items="settingsStore.javaInstallations.map(p => formatJavaPath(p))"
              label="Java 路径"
              :loading="loadingJava"
              persistent-hint
              hint="选择或输入一个Java路径"
              @update:model-value="setJavaPath"
              hide-details
            >
              <template v-slot:append>
                <v-btn
                  icon
                  variant="text"
                  :loading="loadingJava"
                  @click="findJavaInstallations"
                  title="自动查找Java安装"
                >
                  <v-icon>mdi-refresh</v-icon>
                </v-btn>
              </template>
            </v-combobox>
            
            <div v-if="isJavaPathValid" class="mt-3">
              <v-chip color="success" variant="outlined">
                <v-icon start>mdi-check</v-icon>
                Java路径有效
              </v-chip>
            </div>
          </v-card-text>
        </v-card>
      </v-col>
    </v-row>
  </v-container>
</template>
