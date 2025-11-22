<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore } from '../../stores/settings';

const settingsStore = useSettingsStore();
const javaPath = ref('');
const isJavaPathValid = ref(false);
const loadingJava = ref(false);

function formatJavaPath(rawPath: string) {
  if (!rawPath) return '';
  // 统一转换为正斜杠显示
  return rawPath.replace(/\\/g, '/');
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
    // 验证新路径
    isJavaPathValid.value = await invoke('validate_java_path', { path: path });
  } catch (err) {
    console.error('Failed to set Java path:', err);
  }
}

onMounted(async () => {
  await loadJavaPath();
  
  // 只在启动时查找一次Java安装，之后保持状态
  if (!settingsStore.hasFoundJavaInstallations && settingsStore.javaInstallations.length === 0) {
    await findJavaInstallations();
  }
});
</script>

<template>
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
</template>
