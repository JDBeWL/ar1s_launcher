<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import InstanceCard from '../components/instance/InstanceCard.vue';
import InstanceAvatar from '../components/instance/InstanceAvatar.vue';
import { useNotificationStore } from '../stores/notificationStore';
import { getInstanceListCache, setInstanceListCache } from '../stores/instanceListCache';
import { instanceApi } from '../services';
import { useGameLaunch } from '../composables/useGameLaunch';
import { useDiagnosticsExport } from '../composables/useDiagnosticsExport';
import type { GameInstance } from '../types/events';
import { formatLastPlayed, getErrorMessage } from '../utils/format';
import { logError } from '../utils/logger';
import { getInstanceViewMode, getInstanceSortBy } from '../utils/storage';

const route = useRoute();
const router = useRouter();
const notificationStore = useNotificationStore();
const { launchInstance: startInstance } = useGameLaunch();
const { exportDiagnostics } = useDiagnosticsExport();

const instances = ref<GameInstance[]>(getInstanceListCache());
const loading = ref(false);
const refreshing = ref(false);
let lastLoadedAt = 0;
const launchingInstanceId = ref<string | null>(null);
const searchQuery = ref('');
const viewMode = ref<'grid' | 'list'>(getInstanceViewMode());

const renameDialog = ref(false);
const renameInstanceName = ref('');
const currentInstance = ref<GameInstance | null>(null);
const deleteDialog = ref(false);
const deleteConfirmName = ref('');
const sortBy = ref(getInstanceSortBy());

const sortOptions = [
  { title: '最近游玩', value: 'lastPlayed' },
  { title: '名称', value: 'name' },
  { title: '游戏版本', value: 'version' },
];

const filteredInstances = computed(() => {
  let result = instances.value;

  if (searchQuery.value) {
    const query = searchQuery.value.toLowerCase();
    result = result.filter(
      (instance) =>
        instance.name.toLowerCase().includes(query) ||
        instance.gameVersion?.toLowerCase().includes(query) ||
        instance.loaderType?.toLowerCase().includes(query),
    );
  }

  return [...result].sort((a, b) => {
    switch (sortBy.value) {
      case 'name':
        return a.name.localeCompare(b.name);
      case 'version':
        return (a.gameVersion || a.version || '').localeCompare(b.gameVersion || b.version || '');
      case 'lastPlayed':
      default:
        return (b.lastPlayed ?? 0) - (a.lastPlayed ?? 0);
    }
  });
});

async function loadInstances() {
  if (refreshing.value) return;

  refreshing.value = true;
  // 只有列表为空时才铺骨架屏；有旧数据时静默刷新，避免每次进来都闪一下
  if (instances.value.length === 0) {
    loading.value = true;
  }
  try {
    instances.value = await instanceApi.getInstances();
    setInstanceListCache(instances.value);
    lastLoadedAt = Date.now();
  } catch (error) {
    logError('Failed to load instances', error, 'InstanceManagerView');
  } finally {
    loading.value = false;
    refreshing.value = false;
  }
}

function instanceMeta(instance: GameInstance): string {
  const loader = instance.loaderType && instance.loaderType !== 'None' ? instance.loaderType : '';
  const version = instance.gameVersion || instance.version || '';
  const parts: string[] = [];

  if (loader) {
    parts.push(version ? `${loader} · ${version}` : loader);
  } else if (version) {
    parts.push(version);
  }

  if (instance.modCount) {
    parts.push(`${instance.modCount} Mods`);
  }

  if (instance.autoMemoryMb) {
    parts.push(`自动内存 ≈ ${(instance.autoMemoryMb / 1024).toFixed(1)} GB`);
  }

  return parts.join(' · ');
}

async function launchInstance(instance: GameInstance) {
  launchingInstanceId.value = instance.id;
  try {
    if (await startInstance(instance.name)) {
      await loadInstances();
    }
  } finally {
    launchingInstanceId.value = null;
  }
}

async function openInstanceFolder(instance: GameInstance) {
  try {
    await instanceApi.openInstanceFolder(instance.name);
  } catch (error) {
    logError('Failed to open folder', error, 'InstanceManagerView');
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
    await instanceApi.renameInstance(currentInstance.value.name, renameInstanceName.value);
    renameDialog.value = false;
    notificationStore.success('重命名成功');
    await loadInstances();
  } catch (error) {
    logError('Failed to rename instance', error, 'InstanceManagerView');
    notificationStore.error('重命名失败', getErrorMessage(error));
  }
}

function openDeleteDialog(instance: GameInstance) {
  currentInstance.value = instance;
  deleteConfirmName.value = '';
  deleteDialog.value = true;
}

const deleteConfirmed = computed(() => {
  return deleteConfirmName.value === currentInstance.value?.name;
});

async function deleteInstance() {
  if (!currentInstance.value) return;

  try {
    await instanceApi.deleteInstance(currentInstance.value.name);
    deleteDialog.value = false;
    notificationStore.success('删除成功');
    await loadInstances();
  } catch (error) {
    logError('Failed to delete instance', error, 'InstanceManagerView');
    notificationStore.error('删除失败', getErrorMessage(error));
  }
}

function openInstanceDetail(instance: GameInstance) {
  router.push({ name: 'instance-detail', params: { name: instance.name } });
}

watch(viewMode, (v) => localStorage.setItem('instanceViewMode', v));
watch(sortBy, (v) => localStorage.setItem('instanceSortBy', v));

function handleWindowFocus() {
  // 刚拉过就不再拉：切回窗口时列表原地更新，不整页重排
  if (Date.now() - lastLoadedAt < 2000) return;
  void loadInstances();
}

onMounted(() => {
  const searchParam = route.query.search;
  if (searchParam && typeof searchParam === 'string') {
    searchQuery.value = searchParam;
  }
  loadInstances();
  window.addEventListener('focus', handleWindowFocus);
});

onUnmounted(() => {
  window.removeEventListener('focus', handleWindowFocus);
});
</script>

<template>
  <v-container fluid class="page-container pa-4">
    <div class="d-flex align-center justify-space-between mb-4">
      <div class="d-flex align-center flex-grow-1 mr-4">
        <v-text-field
          v-model="searchQuery"
          label="搜索实例"
          density="comfortable"
          variant="outlined"
          hide-details
          clearable
          class="search-field"
          style="max-width: 300px"
        >
          <template #prepend-inner>
            <v-icon size="18" color="on-surface-variant">mdi-magnify</v-icon>
          </template>
        </v-text-field>

        <v-select
          v-model="sortBy"
          aria-label="排序方式"
          :items="sortOptions"
          hide-details
          density="comfortable"
          variant="outlined"
          class="ml-3"
          style="max-width: 180px"
        >
          <template #prepend-inner>
            <v-icon size="18" color="on-surface-variant">mdi-sort</v-icon>
          </template>
        </v-select>

        <v-btn-toggle
          v-model="viewMode"
          mandatory
          border
          divided
          rounded="pill"
          density="comfortable"
          variant="text"
          color="primary"
          class="ml-3"
        >
          <v-btn value="grid" icon aria-label="网格视图">
            <v-icon size="18">mdi-view-grid</v-icon>
          </v-btn>
          <v-btn value="list" icon aria-label="列表视图">
            <v-icon size="18">mdi-view-list</v-icon>
          </v-btn>
        </v-btn-toggle>
      </div>

      <div class="d-flex align-center ga-2">
        <v-btn
          icon
          variant="tonal"
          color="secondary"
          size="small"
          aria-label="刷新列表"
          :loading="refreshing"
          @click="loadInstances"
        >
          <v-icon size="20">mdi-refresh</v-icon>
          <v-tooltip activator="parent" location="bottom">刷新列表</v-tooltip>
        </v-btn>
        <v-btn color="primary" to="/add-instance">
          <v-icon start size="18">mdi-plus</v-icon>
          新建
        </v-btn>
      </div>
    </div>

    <template v-if="loading">
      <v-row density="compact">
        <v-col v-for="i in 6" :key="i" cols="12" sm="6" md="4">
          <v-card color="surface-container" variant="flat" class="skeleton-card">
            <v-card-text class="pa-4">
              <div class="d-flex align-center mb-3">
                <v-skeleton-loader type="avatar" class="mr-3" style="width: 44px; height: 44px" />
                <div class="flex-grow-1">
                  <v-skeleton-loader type="text" style="width: 60%" />
                  <v-skeleton-loader type="text" style="width: 40%" class="mt-1" />
                </div>
              </div>
              <v-skeleton-loader type="text" style="width: 50%" class="mb-3" />
              <v-skeleton-loader type="button" style="width: 100%" />
            </v-card-text>
          </v-card>
        </v-col>
      </v-row>
    </template>

    <div v-else-if="instances.length === 0" class="text-center py-12">
      <v-avatar size="80" color="surface-container-high" class="mb-4">
        <v-icon size="40" color="on-surface-variant">mdi-cube-outline</v-icon>
      </v-avatar>
      <div class="text-title-large font-weight-medium mb-2">没有实例</div>
      <div class="text-body-medium text-on-surface-variant mb-4">创建你的第一个游戏实例</div>
      <v-btn color="primary" to="/add-instance">
        <v-icon start size="18">mdi-plus</v-icon>
        创建实例
      </v-btn>
    </div>

    <div v-else-if="filteredInstances.length === 0" class="text-center py-12">
      <v-avatar size="64" color="surface-container-high" class="mb-3">
        <v-icon size="32" color="on-surface-variant">mdi-magnify-close</v-icon>
      </v-avatar>
      <div class="text-body-large font-weight-medium">没有找到匹配的实例</div>
      <div class="text-body-medium text-on-surface-variant">尝试其他搜索词</div>
    </div>

    <v-row v-else-if="viewMode === 'grid'" density="compact">
      <v-col v-for="instance in filteredInstances" :key="instance.name" cols="12" sm="6" md="4">
        <InstanceCard
          :instance="instance"
          :launching="launchingInstanceId === instance.id"
          @launch="launchInstance"
          @open-folder="openInstanceFolder"
          @open="openInstanceDetail"
          @export-diagnostics="exportDiagnostics"
          @rename="openRenameDialog"
          @delete="openDeleteDialog"
        />
      </v-col>
    </v-row>

    <v-card v-else color="surface-container" variant="flat">
      <v-list lines="two" bg-color="transparent">
        <v-list-item v-for="instance in filteredInstances" :key="instance.name" class="py-3">
          <template #prepend>
            <InstanceAvatar :instance="instance" :size="44" class="mr-3" />
          </template>

          <v-list-item-title class="font-weight-medium">
            {{ instance.name }}
          </v-list-item-title>
          <v-list-item-subtitle>
            {{ instanceMeta(instance) }}
            <span class="ml-2 d-inline-flex align-center">
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
              :loading="launchingInstanceId === instance.id"
              @click="launchInstance(instance)"
            >
              <v-icon start size="16">mdi-play</v-icon>
              启动
            </v-btn>
            <v-menu>
              <template v-slot:activator="{ props }">
                <v-btn icon variant="text" size="small" aria-label="实例操作菜单" v-bind="props">
                  <v-icon size="18">mdi-dots-vertical</v-icon>
                </v-btn>
              </template>
              <v-list density="compact" color="surface-container-high">
                <v-list-item @click="openInstanceFolder(instance)">
                  <template #prepend>
                    <v-icon size="18">mdi-folder-open</v-icon>
                  </template>
                  <v-list-item-title class="text-body-medium">打开文件夹</v-list-item-title>
                </v-list-item>
                <v-list-item @click="openInstanceDetail(instance)">
                  <template #prepend>
                    <v-icon size="18">mdi-view-dashboard-outline</v-icon>
                  </template>
                  <v-list-item-title class="text-body-medium">实例详情</v-list-item-title>
                </v-list-item>
                <v-list-item @click="exportDiagnostics(instance)">
                  <template #prepend>
                    <v-icon size="18">mdi-zip-box-outline</v-icon>
                  </template>
                  <v-list-item-title class="text-body-medium">导出诊断包</v-list-item-title>
                </v-list-item>
                <v-list-item @click="openRenameDialog(instance)">
                  <template #prepend>
                    <v-icon size="18">mdi-pencil</v-icon>
                  </template>
                  <v-list-item-title class="text-body-medium">重命名</v-list-item-title>
                </v-list-item>
                <v-divider class="my-1" />
                <v-list-item @click="openDeleteDialog(instance)">
                  <template #prepend>
                    <v-icon size="18" color="error">mdi-delete</v-icon>
                  </template>
                  <v-list-item-title class="text-body-medium text-error">删除</v-list-item-title>
                </v-list-item>
              </v-list>
            </v-menu>
          </template>
        </v-list-item>
      </v-list>
    </v-card>

    <div v-if="instances.length > 0" class="text-center text-body-small text-on-surface-variant mt-4">
      共 {{ instances.length }} 个实例
      <span v-if="searchQuery && filteredInstances.length !== instances.length">
        ，显示 {{ filteredInstances.length }} 个
      </span>
    </div>

    <v-dialog v-model="renameDialog" max-width="360">
      <v-card color="surface-container-high">
        <v-card-text class="pa-5">
          <div class="text-title-large font-weight-bold mb-4">重命名实例</div>
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

    <v-dialog v-model="deleteDialog" max-width="380">
      <v-card color="surface-container-high">
        <v-card-text class="pa-5">
          <div class="text-title-large font-weight-bold mb-2">删除实例</div>
          <div class="text-body-medium text-on-surface-variant mb-4">
            确定要删除 <strong class="text-on-surface">{{ currentInstance?.name }}</strong> 吗？此操作无法撤销。
          </div>
          <v-text-field
            v-model="deleteConfirmName"
            label="实例名称"
            placeholder="请输入实例名称以确认删除"
            density="comfortable"
            variant="outlined"
            hide-details
            autofocus
          />
        </v-card-text>
        <v-card-actions class="pa-4 pt-0">
          <v-spacer />
          <v-btn variant="text" @click="deleteDialog = false">取消</v-btn>
          <v-btn color="error" :disabled="!deleteConfirmed" @click="deleteInstance">删除</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </v-container>
</template>

<style scoped>
.search-field :deep(.v-field) {
  border-radius: 8px;
}
</style>
