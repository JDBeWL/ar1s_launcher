<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useRouter } from "vue-router";
import InstanceCard from "../components/instance/InstanceCard.vue";
import { useNotificationStore } from "../stores/notificationStore";
import type { GameInstance } from "../types/events";

const router = useRouter();
const notificationStore = useNotificationStore();
const instances = ref<GameInstance[]>([]);
const loading = ref(false);

const renameDialog = ref(false);
const renameInstanceName = ref("");
const currentInstance = ref<GameInstance | null>(null);
const deleteDialog = ref(false);

async function loadInstances() {
  loading.value = true;
  try {
    const result = await invoke<GameInstance[]>("get_instances");
    instances.value = result;
  } catch (error) {
    console.error("Failed to load instances:", error);
  } finally {
    loading.value = false;
  }
}

function launchInstance(instance: GameInstance) {
  router.push({ path: "/", query: { instance: instance.name } });
}

async function openInstanceFolder(instance: GameInstance) {
  try {
    await invoke("open_instance_folder", { instanceName: instance.name });
  } catch (error) {
    console.error("Failed to open folder:", error);
  }
}

function openRenameDialog(instance: GameInstance) {
  currentInstance.value = instance;
  renameInstanceName.value = instance.name;
  renameDialog.value = true;
}

async function renameInstance() {
  if (!currentInstance.value || !renameInstanceName.value) return;
  
  try {
    await invoke("rename_instance", { 
      oldName: currentInstance.value.name, 
      newName: renameInstanceName.value 
    });
    renameDialog.value = false;
    notificationStore.success('重命名成功');
    await loadInstances();
  } catch (error) {
    console.error("Failed to rename instance:", error);
    const errorMessage = error instanceof Error ? error.message : String(error);
    notificationStore.error('重命名失败', errorMessage);
  }
}

function openDeleteDialog(instance: GameInstance) {
  currentInstance.value = instance;
  deleteDialog.value = true;
}

async function deleteInstance() {
  if (!currentInstance.value) return;
  
  try {
    await invoke("delete_instance", { instanceName: currentInstance.value.name });
    deleteDialog.value = false;
    notificationStore.success('删除成功');
    await loadInstances();
  } catch (error) {
    console.error("Failed to delete instance:", error);
    const errorMessage = error instanceof Error ? error.message : String(error);
    notificationStore.error('删除失败', errorMessage);
  }
}

onMounted(() => {
  loadInstances();
});
</script>

<template>
  <v-container fluid class="instance-container pa-4">
    <!-- 页面标题 -->
    <div class="d-flex align-center justify-space-between mb-4">
      <div class="d-flex align-center">
        <v-avatar size="40" class="mr-3 avatar-outlined">
          <v-icon size="20">mdi-folder-multiple</v-icon>
        </v-avatar>
        <div>
          <h1 class="text-h6 font-weight-bold">实例管理</h1>
          <p class="text-body-2 text-medium-emphasis mb-0">管理你的游戏实例</p>
        </div>
      </div>
      <v-btn
        variant="outlined"
        rounded="lg"
        to="/add-instance"
      >
        <v-icon start size="18">mdi-plus</v-icon>
        新建实例
      </v-btn>
    </div>

    <!-- 加载状态 -->
    <div v-if="loading" class="text-center py-12">
      <v-progress-circular indeterminate size="40" />
      <div class="text-body-2 text-medium-emphasis mt-3">加载实例中...</div>
    </div>

    <!-- 空状态 -->
    <div v-else-if="instances.length === 0" class="text-center py-12">
      <v-avatar size="80" class="mb-4 avatar-outlined">
        <v-icon size="40">mdi-cube-outline</v-icon>
      </v-avatar>
      <div class="text-h6 font-weight-medium mb-1">没有找到实例</div>
      <div class="text-body-2 text-medium-emphasis mb-4">
        创建你的第一个 Minecraft 实例来开始游戏
      </div>
      <v-btn
        variant="outlined"
        rounded="lg"
        to="/add-instance"
      >
        <v-icon start size="18">mdi-plus</v-icon>
        创建新实例
      </v-btn>
    </div>

    <!-- 实例列表 -->
    <v-row v-else dense>
      <v-col
        v-for="instance in instances"
        :key="instance.name"
        cols="12"
        sm="6"
        md="4"
      >
        <InstanceCard
          :instance="instance"
          @launch="launchInstance"
          @open-folder="openInstanceFolder"
          @rename="openRenameDialog"
          @delete="openDeleteDialog"
        />
      </v-col>
    </v-row>

    <!-- 重命名对话框 -->
    <v-dialog v-model="renameDialog" max-width="360">
      <v-card rounded="xl">
        <v-card-text class="pa-5">
          <div class="text-center mb-4">
            <v-avatar size="56" class="mb-3 avatar-outlined">
              <v-icon size="28">mdi-pencil</v-icon>
            </v-avatar>
            <div class="text-h6 font-weight-bold">重命名实例</div>
          </div>
          <v-text-field
            v-model="renameInstanceName"
            label="新名称"
            autofocus
            variant="outlined"
            density="compact"
            rounded="lg"
            hide-details
          />
        </v-card-text>
        <v-card-actions class="pa-4 pt-0">
          <v-btn
            variant="outlined"
            rounded="lg"
            class="flex-grow-1"
            @click="renameDialog = false"
          >
            取消
          </v-btn>
          <v-btn
            variant="flat"
            rounded="lg"
            class="flex-grow-1"
            @click="renameInstance"
          >
            确定
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <!-- 删除确认对话框 -->
    <v-dialog v-model="deleteDialog" max-width="360">
      <v-card rounded="xl">
        <v-card-text class="pa-5">
          <div class="text-center mb-4">
            <v-avatar size="56" color="error" variant="tonal" class="mb-3">
              <v-icon size="28">mdi-alert</v-icon>
            </v-avatar>
            <div class="text-h6 font-weight-bold">删除实例</div>
          </div>
          <div class="text-body-2 text-center">
            确定要删除实例 <strong>"{{ currentInstance?.name }}"</strong> 吗？
          </div>
          <div class="text-caption text-medium-emphasis text-center mt-2">
            此操作无法撤销，所有数据将被永久删除
          </div>
        </v-card-text>
        <v-card-actions class="pa-4 pt-0">
          <v-btn
            variant="outlined"
            rounded="lg"
            class="flex-grow-1"
            @click="deleteDialog = false"
          >
            取消
          </v-btn>
          <v-btn
            variant="flat"
            rounded="lg"
            color="error"
            class="flex-grow-1"
            @click="deleteInstance"
          >
            删除
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </v-container>
</template>

<style scoped>
.instance-container {
  max-width: 900px;
  margin: 0 auto;
}

.avatar-outlined {
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
}
</style>
