<script setup lang="ts">
import { ref, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from '@tauri-apps/api/event';
import { useSettingsStore } from '../../stores/settings';

const settingsStore = useSettingsStore();
const gameDir = ref('');
const versionIsolation = ref(true);
const downloadThreads = ref(32);

// 加载已保存的游戏目录
async function loadGameDir() {
  try {
    const dir = await invoke('get_game_dir');
    gameDir.value = dir as string;
  } catch (err) {
    console.error('Failed to get game directory:', err);
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

onMounted(async () => {
  await loadGameDir();
  await loadDownloadThreads();
  await loadVersionIsolation();
  await settingsStore.loadDownloadMirror();
  
  // 监听游戏目录变更事件
  await listen('game-dir-changed', (event) => {
    gameDir.value = event.payload as string;
  });
});
</script>

<template>
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
            :max="64"
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
</template>
