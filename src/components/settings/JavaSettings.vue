<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useSettingsStore } from '../../stores/settings';

const settingsStore = useSettingsStore();
const javaPath = ref('');
const isJavaPathValid = ref(false);
const loadingJava = ref(false);
const javaVersion = ref('');

const formattedJavaPath = computed(() => {
  if (!javaPath.value) return '';
  return javaPath.value.replace(/\\/g, '/');
});

async function loadJavaPath() {
  try {
    javaPath.value = (await invoke('load_config_key', { key: 'javaPath' })) as string;
    if (javaPath.value) {
      isJavaPathValid.value = await invoke('validate_java_path', { path: javaPath.value });
      if (isJavaPathValid.value) {
        await getJavaVersion();
      }
    }
  } catch (error) {
    console.error('Failed to load Java path:', error);
  }
}

async function getJavaVersion() {
  try {
    const version = await invoke('get_java_version', { path: javaPath.value });
    javaVersion.value = version as string;
  } catch {
    javaVersion.value = '';
  }
}

async function findJavaInstallations() {
  try {
    loadingJava.value = true;
    await settingsStore.findJavaInstallations();
    
    if (settingsStore.javaInstallations.length > 0 && !javaPath.value) {
      await selectJavaPath(settingsStore.javaInstallations[0]);
    }
  } catch (err) {
    console.error('Failed to find Java installations:', err);
  } finally {
    loadingJava.value = false;
  }
}

async function selectJavaPath(path: string) {
  try {
    javaPath.value = path;
    await invoke('save_config_key', { key: 'javaPath', value: path });
    isJavaPathValid.value = await invoke('validate_java_path', { path });
    if (isJavaPathValid.value) {
      await getJavaVersion();
    }
  } catch (err) {
    console.error('Failed to set Java path:', err);
  }
}

async function browseJavaPath() {
  try {
    const selected = await open({
      multiple: false,
      title: '选择 Java 可执行文件',
      filters: [{
        name: 'Java',
        extensions: ['exe', '']
      }]
    });
    if (selected) {
      await selectJavaPath(selected as string);
    }
  } catch (err) {
    console.error('Failed to browse Java path:', err);
  }
}

onMounted(async () => {
  await loadJavaPath();
  
  if (!settingsStore.hasFoundJavaInstallations && settingsStore.javaInstallations.length === 0) {
    await findJavaInstallations();
  }
});
</script>

<template>
  <div class="settings-group">
    <!-- 标题 -->
    <div class="group-header mb-5">
      <div class="d-flex align-center">
        <v-avatar size="48" color="primary-container" class="mr-3">
          <v-icon size="24" color="on-primary-container">mdi-language-java</v-icon>
        </v-avatar>
        <div>
          <h2 class="text-h6 font-weight-bold">Java 配置</h2>
          <p class="text-body-2 text-on-surface-variant mb-0">选择用于启动游戏的 Java 运行时</p>
        </div>
      </div>
    </div>

    <!-- Java 路径选择 -->
    <v-card color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center justify-space-between mb-4">
          <div class="d-flex align-center">
            <v-icon class="mr-2" color="on-surface-variant">mdi-file-cog-outline</v-icon>
            <span class="text-subtitle-1 font-weight-medium">Java 路径</span>
          </div>
          <div class="d-flex ga-2">
            <v-btn
              variant="tonal"
              color="primary"
              size="small"
              :loading="loadingJava"
              @click="findJavaInstallations"
            >
              <v-icon start size="18">mdi-magnify</v-icon>
              自动查找
            </v-btn>
            <v-btn
              variant="tonal"
              color="secondary"
              size="small"
              @click="browseJavaPath"
            >
              <v-icon start size="18">mdi-folder-open-outline</v-icon>
              浏览
            </v-btn>
          </div>
        </div>

        <!-- 已检测到的 Java -->
        <div v-if="settingsStore.javaInstallations.length > 0" class="mb-4">
          <div class="text-body-2 text-on-surface-variant mb-2">检测到的 Java 安装：</div>
          <v-list density="compact" class="java-list" bg-color="surface-container-high">
            <v-list-item
              v-for="(path, index) in settingsStore.javaInstallations"
              :key="index"
              :active="javaPath === path"
              @click="selectJavaPath(path)"
            >
              <template #prepend>
                <v-icon :color="javaPath === path ? 'primary' : 'on-surface-variant'">
                  {{ javaPath === path ? 'mdi-radiobox-marked' : 'mdi-radiobox-blank' }}
                </v-icon>
              </template>
              <v-list-item-title class="text-body-2 font-mono">
                {{ path.replace(/\\/g, '/') }}
              </v-list-item-title>
            </v-list-item>
          </v-list>
        </div>

        <!-- 当前选择 -->
        <div v-if="javaPath" class="current-java pa-4">
          <div class="d-flex align-center justify-space-between">
            <div>
              <div class="text-body-2 text-on-surface-variant">当前选择</div>
              <div class="text-body-1 font-mono">{{ formattedJavaPath }}</div>
            </div>
            <v-chip
              :color="isJavaPathValid ? 'success' : 'error'"
              variant="tonal"
              size="small"
            >
              <v-icon start size="16">
                {{ isJavaPathValid ? 'mdi-check-circle' : 'mdi-alert-circle' }}
              </v-icon>
              {{ isJavaPathValid ? '有效' : '无效' }}
            </v-chip>
          </div>
          <div v-if="javaVersion" class="text-caption text-on-surface-variant mt-1">
            版本: {{ javaVersion }}
          </div>
        </div>

        <!-- 未选择提示 -->
        <v-alert
          v-else
          color="warning-container"
          density="compact"
        >
          <template #prepend>
            <v-icon color="on-warning-container">mdi-alert-outline</v-icon>
          </template>
          <span class="text-on-warning-container">未检测到 Java，请点击"自动查找"或手动选择 Java 路径</span>
        </v-alert>
      </v-card-text>
    </v-card>

    <!-- Java 提示 -->
    <v-alert color="tertiary-container">
      <template #prepend>
        <v-icon color="on-tertiary-container">mdi-information-outline</v-icon>
      </template>
      <template #title>
        <span class="text-body-2 font-weight-medium text-on-tertiary-container">Java 版本建议</span>
      </template>
      <ul class="text-body-2 pl-4 mb-0 mt-1 text-on-tertiary-container">
        <li>Minecraft 1.17+ 需要 Java 17 或更高版本</li>
        <li>Minecraft 1.16.5 及以下版本建议使用 Java 8</li>
        <li>推荐使用 Adoptium (Eclipse Temurin) 或 Azul Zulu</li>
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

.java-list {
  max-height: 200px;
  overflow-y: auto;
}

.current-java {
  background: rgb(var(--v-theme-surface-container-high));
  border-radius: 12px;
}

.font-mono {
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 0.875rem;
}
</style>
