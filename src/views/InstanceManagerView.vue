<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useRouter } from "vue-router";
import InstanceCard from "../components/instance/InstanceCard.vue";
import { useNotificationStore } from "../stores/notificationStore";
import type { GameInstance } from "../types/events";
import { formatLastPlayed, getLoaderIcon, getErrorMessage } from "../utils/format";

const router = useRouter();
const notificationStore = useNotificationStore();
const instances = ref<GameInstance[]>([]);
const loading = ref(false);
const searchQuery = ref('');
const viewMode = ref<'grid' | 'list'>('grid');

const renameDialog = ref(false);
const renameInstanceName = ref("");
const currentInstance = ref<GameInstance | null>(null);
const deleteDialog = ref(false);

// 过滤后的实例列表
const filteredInstances = computed(() => {
  if (!searchQuery.value) return instances.value;
  const query = searchQuery.value.toLowerCase();
  return instances.value.filter(instance => 
    instance.name.toLowerCase().includes(query) ||
    instance.gameVersion?.toLowerCase().includes(query) ||
    instance.loaderType?.toLowerCase().includes(query)
  );
});

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
    notificationStore.error('重命名失败', getErrorMessage(error));
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
    notificationStore.error('删除失败', getErrorMessage(error));
  }
}

onMounted(() => {
  loadInstances();
});
</script>

<template>
  <v-container fluid class="instance-container pa-4">
    <!-- 顶部工具栏 -->
    <div class="d-flex align-center justify-space-between mb-4">
      <div class="d-flex align-center flex-grow-1 mr-4">
        <v-text-field
          v-model="searchQuery"
          placeholder="搜索实例..."
          density="comfortable"
          variant="outlined"
          hide-details
          clearable
          class="search-field"
          style="max-width: 300px;"
        >
          <template #prepend-inner>
            <v-icon size="18" color="on-surface-variant">mdi-magnify</v-icon>
          </template>
        </v-text-field>
        
        <v-btn-toggle
          v-model="viewMode"
          mandatory
          density="comfortable"
          class="ml-3"
          variant="outlined"
        >
          <v-btn value="grid" size="small">
            <v-icon size="18">mdi-view-grid</v-icon>
          </v-btn>
          <v-btn value="list" size="small">
            <v-icon size="18">mdi-view-list</v-icon>
          </v-btn>
        </v-btn-toggle>
      </div>

      <v-btn color="primary" to="/add-instance">
        <v-icon start size="18">mdi-plus</v-icon>
        新建
      </v-btn>
    </div>

    <!-- 加载状态 -->
    <div v-if="loading" class="text-center py-12">
      <v-progress-circular indeterminate size="40" color="primary" />
      <div class="text-body-2 text-on-surface-variant mt-3">加载中...</div>
    </div>

    <!-- 空状态 -->
    <div v-else-if="instances.length === 0" class="text-center py-12">
      <v-avatar size="80" color="surface-container-high" class="mb-4">
        <v-icon size="40" color="on-surface-variant">mdi-cube-outline</v-icon>
      </v-avatar>
      <div class="text-h6 font-weight-medium mb-2">没有实例</div>
      <div class="text-body-2 text-on-surface-variant mb-4">
        创建你的第一个游戏实例
      </div>
      <v-btn color="primary" to="/add-instance">
        <v-icon start size="18">mdi-plus</v-icon>
        创建实例
      </v-btn>
    </div>

    <!-- 搜索无结果 -->
    <div v-else-if="filteredInstances.length === 0" class="text-center py-12">
      <v-avatar size="64" color="surface-container-high" class="mb-3">
        <v-icon size="32" color="on-surface-variant">mdi-magnify-close</v-icon>
      </v-avatar>
      <div class="text-body-1 font-weight-medium">没有找到匹配的实例</div>
      <div class="text-body-2 text-on-surface-variant">尝试其他搜索词</div>
    </div>

    <!-- 网格视图 -->
    <v-row v-else-if="viewMode === 'grid'" dense>
      <v-col
        v-for="instance in filteredInstances"
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

    <!-- 列表视图 -->
    <v-card v-else color="surface-container" variant="flat">
      <v-list lines="two" bg-color="transparent">
        <v-list-item
          v-for="instance in filteredInstances"
          :key="instance.name"
          class="py-3"
        >
          <template #prepend>
            <v-avatar size="44" color="primary-container" class="mr-3">
              <v-icon size="22" color="on-primary-container">{{ getLoaderIcon(instance.loaderType) }}</v-icon>
            </v-avatar>
          </template>

          <v-list-item-title class="font-weight-medium">
            {{ instance.name }}
          </v-list-item-title>
          <v-list-item-subtitle>
            <span v-if="instance.loaderType && instance.loaderType !== 'None'">{{ instance.loaderType }} · </span>
            {{ instance.gameVersion || instance.version }}
            <span class="ml-2">
              <v-icon size="12" class="mr-1">mdi-clock-outline</v-icon>
              {{ formatLastPlayed(instance.lastPlayed) }}
            </span>
          </v-list-item-subtitle>

          <template #append>
            <v-btn
              variant="tonal"
              color="primary"
              size="small"
              class="mr-2"
              @click="launchInstance(instance)"
            >
              <v-icon start size="16">mdi-play</v-icon>
              启动
            </v-btn>
            <v-menu>
              <template v-slot:activator="{ props }">
                <v-btn icon variant="text" size="small" v-bind="props">
                  <v-icon size="18">mdi-dots-vertical</v-icon>
                </v-btn>
              </template>
              <v-list density="compact" color="surface-container-high">
                <v-list-item @click="openInstanceFolder(instance)">
                  <template #prepend>
                    <v-icon size="18">mdi-folder-open</v-icon>
                  </template>
                  <v-list-item-title class="text-body-2">打开文件夹</v-list-item-title>
                </v-list-item>
                <v-list-item @click="openRenameDialog(instance)">
                  <template #prepend>
                    <v-icon size="18">mdi-pencil</v-icon>
                  </template>
                  <v-list-item-title class="text-body-2">重命名</v-list-item-title>
                </v-list-item>
                <v-divider class="my-1" />
                <v-list-item @click="openDeleteDialog(instance)">
                  <template #prepend>
                    <v-icon size="18" color="error">mdi-delete</v-icon>
                  </template>
                  <v-list-item-title class="text-body-2 text-error">删除</v-list-item-title>
                </v-list-item>
              </v-list>
            </v-menu>
          </template>
        </v-list-item>
      </v-list>
    </v-card>

    <!-- 统计信息 -->
    <div v-if="instances.length > 0" class="text-center text-caption text-on-surface-variant mt-4">
      共 {{ instances.length }} 个实例
      <span v-if="searchQuery && filteredInstances.length !== instances.length">
        ，显示 {{ filteredInstances.length }} 个
      </span>
    </div>

    <!-- 重命名对话框 -->
    <v-dialog v-model="renameDialog" max-width="360">
      <v-card color="surface-container-high">
        <v-card-text class="pa-5">
          <div class="text-h6 font-weight-bold mb-4">重命名实例</div>
          <v-text-field
            v-model="renameInstanceName"
            label="新名称"
            autofocus
            density="comfortable"
            variant="outlined"
            hide-details
          />
        </v-card-text>
        <v-card-actions class="pa-4 pt-0">
          <v-spacer />
          <v-btn variant="text" @click="renameDialog = false">取消</v-btn>
          <v-btn color="primary" @click="renameInstance">确定</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <!-- 删除确认对话框 -->
    <v-dialog v-model="deleteDialog" max-width="360">
      <v-card color="surface-container-high">
        <v-card-text class="pa-5">
          <div class="text-h6 font-weight-bold mb-2">删除实例</div>
          <div class="text-body-2 text-on-surface-variant">
            确定要删除 <strong class="text-on-surface">{{ currentInstance?.name }}</strong> 吗？此操作无法撤销。
          </div>
        </v-card-text>
        <v-card-actions class="pa-4 pt-0">
          <v-spacer />
          <v-btn variant="text" @click="deleteDialog = false">取消</v-btn>
          <v-btn color="error" @click="deleteInstance">删除</v-btn>
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

.search-field :deep(.v-field) {
  border-radius: 8px;
}
</style>
