<script setup lang="ts">
import { ref, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import VersionSelector from "../components/home/VersionSelector.vue";
import LaunchCard from "../components/home/LaunchCard.vue";
import GameSettingsCard from "../components/home/GameSettingsCard.vue";
import QuickNavCard from "../components/home/QuickNavCard.vue";
import { useVersionManager } from "../composables/useVersionManager";
import { useGameLaunch } from "../composables/useGameLaunch";

const {
  installedVersions,
  selectedVersion,
  loading: versionLoading,
  gameDir,
  loadGameDir,
  loadInstalledVersions,
  initListeners
} = useVersionManager();

const { 
  loading: launchLoading, 
  launchGame,
  isRepairing,
  repairProgress 
} = useGameLaunch();

const username = ref('')
const offlineMode = ref(true)

// Load saved username from backend
async function loadUsername() {
  try {
    const savedUsername = await invoke('get_saved_username');
    if (savedUsername) {
      username.value = savedUsername as string;
    }
  } catch (err) {
    console.error("Failed to load username:", err);
  }
}

// Save username to backend
async function saveUsername(newName: string) {
  try {
    await invoke('set_saved_username', { username: newName });
  } catch (err) {
    console.error("Failed to save username:", err);
  }
}

// Watch for username changes and save them
watch(username, (newName) => {
  if (newName !== null && newName !== undefined) {
    saveUsername(newName);
  }
});

// Handle launch
async function handleLaunch() {
  await launchGame(
    selectedVersion.value,
    username.value,
    offlineMode.value,
    gameDir.value
  );
}

onMounted(async () => {
  await loadGameDir();
  await loadUsername();
  await initListeners();
});
</script>

<template>
  <v-container>
    <v-row>
      <!-- 左侧：版本选择和设置区域 -->
      <v-col cols="12" md="7" lg="8">
        <VersionSelector
          v-model:selectedVersion="selectedVersion"
          :installedVersions="installedVersions"
          :loading="versionLoading"
          @refresh="loadInstalledVersions"
        />

        <GameSettingsCard
          v-model:username="username"
          v-model:offlineMode="offlineMode"
        />
      </v-col>

      <!-- 右侧：启动游戏和快速导航 -->
      <v-col cols="12" md="5" lg="4">
        <LaunchCard
          :selectedVersion="selectedVersion"
          :loading="launchLoading"
          :isRepairing="isRepairing"
          :repairProgress="repairProgress"
          @launch="handleLaunch"
        />

        <QuickNavCard />
      </v-col>
    </v-row>
  </v-container>
</template>

<style scoped>
/* 响应式布局调整 */
@media (max-width: 960px) {
  .v-container {
    padding: 16px;
  }
}

@media (max-width: 800px) {
  .v-col-md-5,
  .v-col-md-7 {
    flex: 0 0 100%;
    max-width: 100%;
  }
}

/* 深色模式适配 */
:deep(.v-theme--dark) .v-card {
  background-color: #1e1e1e;
}
</style>