<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import { useRoute } from 'vue-router';
import InstanceCard from '../components/instance/InstanceCard.vue';
import InstanceAvatar from '../components/instance/InstanceAvatar.vue';
import { useNotificationStore } from '../stores/notificationStore';
import { configApi, instanceApi } from '../services';
import { useGameLaunch } from '../composables/useGameLaunch';
import type { GameInstance, ModInfo } from '../types/events';
import { formatLastPlayed, getErrorMessage } from '../utils/format';
import { logError } from '../utils/logger';
import { getInstanceViewMode, getInstanceSortBy } from '../utils/storage';

const route = useRoute();
const notificationStore = useNotificationStore();
const { launchInstance: startInstance } = useGameLaunch();
const instances = ref<GameInstance[]>([]);
const loading = ref(false);
const launchingInstanceId = ref<string | null>(null);
const searchQuery = ref('');
const viewMode = ref<'grid' | 'list'>(getInstanceViewMode());

const renameDialog = ref(false);
const renameInstanceName = ref('');
const currentInstance = ref<GameInstance | null>(null);
const deleteDialog = ref(false);
const deleteConfirmName = ref('');
const sortBy = ref(getInstanceSortBy());

const modDialog = ref(false);
const mods = ref<ModInfo[]>([]);
const modsLoading = ref(false);
const modsError = ref('');
const modBusy = ref<string | null>(null);
const modDeleteDialog = ref(false);
const modToDelete = ref<ModInfo | null>(null);
// 关闭版本隔离时模组在共享目录里，启停/删除会影响所有实例
const versionIsolation = ref(true);

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
  loading.value = true;
  try {
    instances.value = await instanceApi.getInstances();
  } catch (error) {
    logError('Failed to load instances', error, 'InstanceManagerView');
  } finally {
    loading.value = false;
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

async function openModDialog(instance: GameInstance) {
  currentInstance.value = instance;
  mods.value = [];
  modDialog.value = true;
  await loadMods();
}

async function loadMods() {
  if (!currentInstance.value) return;
  modsLoading.value = true;
  modsError.value = '';
  try {
    mods.value = await instanceApi.getMods(currentInstance.value.name);
  } catch (error) {
    logError('Failed to load mods', error, 'InstanceManagerView');
    modsError.value = getErrorMessage(error);
  } finally {
    modsLoading.value = false;
  }
}

async function toggleMod(mod: ModInfo) {
  if (!currentInstance.value) return;
  modBusy.value = mod.fileName;
  try {
    await instanceApi.setModEnabled(currentInstance.value.name, mod.fileName, !mod.enabled);
    await loadMods();
    await loadInstances();
  } catch (error) {
    logError('Failed to toggle mod', error, 'InstanceManagerView');
    notificationStore.error('模组操作失败', getErrorMessage(error));
  } finally {
    modBusy.value = null;
  }
}

function askDeleteMod(mod: ModInfo) {
  modToDelete.value = mod;
  modDeleteDialog.value = true;
}

async function deleteMod() {
  if (!currentInstance.value || !modToDelete.value) return;
  const fileName = modToDelete.value.fileName;
  modBusy.value = fileName;
  try {
    await instanceApi.deleteMod(currentInstance.value.name, fileName);
    modDeleteDialog.value = false;
    modToDelete.value = null;
    await loadMods();
    await loadInstances();
  } catch (error) {
    logError('Failed to delete mod', error, 'InstanceManagerView');
    notificationStore.error('删除模组失败', getErrorMessage(error));
  } finally {
    modBusy.value = null;
  }
}

function formatModSize(bytes: number): string {
  return bytes < 1024 * 1024 ? `${Math.max(1, Math.round(bytes / 1024))} KB` : `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

watch(viewMode, (v) => localStorage.setItem('instanceViewMode', v));
watch(sortBy, (v) => localStorage.setItem('instanceSortBy', v));

async function loadIsolationSetting() {
  try {
    const value = await configApi.loadConfigKey('versionIsolation');
    if (value !== null) versionIsolation.value = value === 'true';
  } catch (err) {
    logError('Failed to load isolation setting', err, 'InstanceManagerView');
  }
}

function handleWindowFocus() {
  void loadInstances();
  void loadIsolationSetting();
}

onMounted(() => {
  const searchParam = route.query.search;
  if (searchParam && typeof searchParam === 'string') {
    searchQuery.value = searchParam;
  }
  void loadIsolationSetting();
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
          :loading="loading"
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
          @manage-mods="openModDialog"
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
                <v-list-item @click="openModDialog(instance)">
                  <template #prepend>
                    <v-icon size="18">mdi-puzzle-outline</v-icon>
                  </template>
                  <v-list-item-title class="text-body-medium">Mod 管理</v-list-item-title>
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

    <v-dialog v-model="modDialog" max-width="640" scrollable>
      <v-card color="surface-container-high">
        <v-card-title class="d-flex align-center pa-4 pb-3">
          <v-icon size="20" class="mr-2" color="on-surface-variant">mdi-puzzle-outline</v-icon>
          <span class="text-title-medium font-weight-bold">Mod 管理</span>
          <span v-if="currentInstance" class="text-body-medium text-on-surface-variant ml-2">
            · {{ currentInstance.name }}
          </span>
          <v-spacer />
          <v-btn
            icon
            size="small"
            variant="text"
            aria-label="重新加载模组列表"
            :loading="modsLoading"
            @click="loadMods"
          >
            <v-icon size="18">mdi-refresh</v-icon>
          </v-btn>
          <v-btn icon size="small" variant="text" aria-label="关闭" @click="modDialog = false">
            <v-icon size="18">mdi-close</v-icon>
          </v-btn>
        </v-card-title>
        <v-divider />

        <v-card-text class="pa-0" style="min-height: 160px">
          <div v-if="modsLoading && mods.length === 0" class="d-flex align-center justify-center pa-8">
            <v-progress-circular indeterminate size="28" color="primary" />
          </div>
          <div v-else-if="modsError" class="text-center pa-8">
            <div class="text-body-medium text-error mb-3">{{ modsError }}</div>
            <v-btn size="small" variant="tonal" color="primary" @click="loadMods">重试</v-btn>
          </div>
          <div v-else-if="mods.length === 0" class="text-center pa-8 text-on-surface-variant">
            <v-icon size="36" class="mb-2" color="on-surface-variant">mdi-puzzle-outline</v-icon>
            <div class="text-body-medium">这个实例还没有模组</div>
          </div>
          <v-list v-else lines="two" density="compact">
            <v-list-item v-for="mod in mods" :key="mod.fileName">
              <template #prepend>
                <v-icon size="20" color="on-surface-variant">mdi-puzzle-outline</v-icon>
              </template>
              <v-list-item-title class="text-body-medium" :class="{ 'text-on-surface-variant': !mod.enabled }">
                {{ mod.displayName }}
              </v-list-item-title>
              <v-list-item-subtitle class="text-body-small">
                <template v-if="mod.version">v{{ mod.version }} · </template>
                {{ formatModSize(mod.size) }} · {{ mod.fileName }}
              </v-list-item-subtitle>
              <template #append>
                <div class="d-flex align-center ga-1">
                  <v-switch
                    :model-value="mod.enabled"
                    :loading="modBusy === mod.fileName"
                    :disabled="modBusy === mod.fileName"
                    :aria-label="mod.enabled ? `禁用 ${mod.displayName}` : `启用 ${mod.displayName}`"
                    hide-details
                    inset
                    density="compact"
                    color="primary"
                    @update:model-value="toggleMod(mod)"
                  />
                  <v-btn
                    icon
                    size="small"
                    variant="text"
                    :aria-label="`删除 ${mod.displayName}`"
                    :disabled="modBusy === mod.fileName"
                    @click="askDeleteMod(mod)"
                  >
                    <v-icon size="18">mdi-delete-outline</v-icon>
                  </v-btn>
                </div>
              </template>
            </v-list-item>
          </v-list>
        </v-card-text>

        <v-divider />
        <v-card-actions class="pa-3">
          <span class="text-body-small text-on-surface-variant">
            {{
              versionIsolation
                ? '禁用只是把文件改名为 .jar.disabled，不会删除'
                : '共享目录：这里启停或删除会影响所有实例'
            }}
          </span>
          <v-spacer />
          <v-btn variant="text" @click="modDialog = false">关闭</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <v-dialog v-model="modDeleteDialog" max-width="380">
      <v-card color="surface-container-high">
        <v-card-text class="pa-5">
          <div class="text-title-large font-weight-bold mb-2">删除模组</div>
          <div class="text-body-medium text-on-surface-variant">
            确定要删除 <strong class="text-on-surface">{{ modToDelete?.displayName }}</strong> 吗？ 文件
            {{ modToDelete?.fileName }} 会直接从磁盘移除。
          </div>
          <div v-if="!versionIsolation" class="text-body-small text-error mt-3">
            当前关闭了版本隔离，这个文件在共享 mods 目录里，删除后所有实例都不再加载它。
          </div>
        </v-card-text>
        <v-card-actions class="pa-4 pt-0">
          <v-spacer />
          <v-btn variant="text" @click="modDeleteDialog = false">取消</v-btn>
          <v-btn color="error" :loading="modBusy === modToDelete?.fileName" @click="deleteMod">删除</v-btn>
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
