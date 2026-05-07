<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { useRoute } from "vue-router";
import InstanceCard from "../components/instance/InstanceCard.vue";
import { useNotificationStore } from "../stores/notificationStore";
import { useAuthStore } from "../stores/authStore";
import { useSettingsStore } from "../stores/settings";
import { instanceApi, javaApi } from "../services";
import type { GameInstance, JavaCompatibilityResult } from "../types/events";
import { formatLastPlayed, getLoaderIcon, getLoaderColor, getErrorMessage } from "../utils/format";
import { logError } from "../utils/logger";
import { getInstanceViewMode, getInstanceSortBy } from "../utils/storage";

const route = useRoute();
const notificationStore = useNotificationStore();
const authStore = useAuthStore();
const settingsStore = useSettingsStore();
const instances = ref<GameInstance[]>([]);
const loading = ref(false);
const launchingInstanceId = ref<string | null>(null);
const searchQuery = ref('');
const viewMode = ref<'grid' | 'list'>(getInstanceViewMode());

const renameDialog = ref(false);
const renameInstanceName = ref("");
const currentInstance = ref<GameInstance | null>(null);
const deleteDialog = ref(false);
const deleteConfirmName = ref('');
const sortBy = ref(getInstanceSortBy());

const sortOptions = [
  { title: '最近游玩', value: 'lastPlayed' },
  { title: '名称', value: 'name' },
  { title: '游戏版本', value: 'version' },
];

// 过滤和排序后的实例列表
const filteredInstances = computed(() => {
  let result = instances.value;
  
  if (searchQuery.value) {
    const query = searchQuery.value.toLowerCase();
    result = result.filter(instance => 
      instance.name.toLowerCase().includes(query) ||
      instance.gameVersion?.toLowerCase().includes(query) ||
      instance.loaderType?.toLowerCase().includes(query)
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
    logError("Failed to load instances", error, 'InstanceManagerView')
  } finally {
    loading.value = false;
  }
}

async function checkAndResolveJava(version: string): Promise<{ shouldLaunch: boolean; overrideJavaPath?: string }> {
  let result: JavaCompatibilityResult;
  try {
    result = await javaApi.checkJavaCompatibility(version);
  } catch {
    return { shouldLaunch: true };
  }

  if (result.compatible || result.autoMatchEnabled) {
    return { shouldLaunch: true };
  }

  const currentVer = result.currentJavaVersion != null ? `Java ${result.currentJavaVersion}` : '未知';
  const requiredVer = `Java ${result.requiredJavaVersion}`;

  let content = `当前 Java 版本 (${currentVer}) 不满足 Minecraft ${version} 的要求 (需要 ${requiredVer})。\n\n请选择如何处理：`;

  const options: Array<{ id: string; label: string; color?: string; variant?: 'elevated' | 'outlined' | 'text' | 'flat' | 'tonal' | 'plain' }> = [];

  if (result.recommendedJavaPath) {
    options.push({
      id: 'temp',
      label: `临时使用 Java ${result.recommendedJavaVersion}`,
      color: 'primary',
      variant: 'elevated',
    });
  }

  options.push({
    id: 'auto',
    label: '开启自动匹配',
    color: 'success',
    variant: 'tonal',
  });

  options.push({
    id: 'manual',
    label: '手动选择 Java',
    color: 'warning',
    variant: 'outlined',
  });

  options.push({
    id: 'continue',
    label: '继续启动',
    color: 'info',
    variant: 'text',
  });

  const selected = await notificationStore.choice(
    'Java 版本提示',
    content,
    options,
    'warning'
  );

  if (selected === null) {
    return { shouldLaunch: false };
  }

  if (selected === 'continue') {
    return { shouldLaunch: true };
  }

  if (selected === 'temp' && result.recommendedJavaPath) {
    return { shouldLaunch: true, overrideJavaPath: result.recommendedJavaPath };
  }

  if (selected === 'auto') {
    settingsStore.autoMatchJava = true;
    await settingsStore.saveAutoMatchJava();
    return { shouldLaunch: false };
  }

  if (selected === 'manual') {
    notificationStore.warning('请在设置中手动选择兼容的 Java 路径');
    return { shouldLaunch: false };
  }

  return { shouldLaunch: false };
}

async function launchInstance(instance: GameInstance) {
  if (!authStore.isLoggedIn) {
    notificationStore.warning('请先设置玩家名称或登录 Microsoft 账户');
    return;
  }

  launchingInstanceId.value = instance.id;
  try {
    if (authStore.authType === 'microsoft' && authStore.msLoggedIn && authStore.isTokenExpired) {
      await authStore.tryRefreshMicrosoftToken();
    }

    const javaResolve = await checkAndResolveJava(instance.name);

    if (!javaResolve.shouldLaunch) {
      return;
    }

    await instanceApi.launchInstance(instance.name, javaResolve.overrideJavaPath);
    notificationStore.success('启动成功', `${instance.name} 正在启动`);
    await loadInstances();
  } catch (error) {
    logError("Failed to launch instance", error, 'InstanceManagerView');
    notificationStore.error('启动失败', getErrorMessage(error), true);
  } finally {
    launchingInstanceId.value = null;
  }
}

async function openInstanceFolder(instance: GameInstance) {
  try {
    await instanceApi.openInstanceFolder(instance.name);
  } catch (error) {
    logError("Failed to open folder", error, 'InstanceManagerView')
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
    logError("Failed to rename instance", error, 'InstanceManagerView')
    notificationStore.error('重命名失败', getErrorMessage(error))
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
    logError("Failed to delete instance", error, 'InstanceManagerView')
    notificationStore.error('删除失败', getErrorMessage(error))
  }
}

// 持久化用户偏好
watch(viewMode, (v) => localStorage.setItem('instanceViewMode', v));
watch(sortBy, (v) => localStorage.setItem('instanceSortBy', v));

onMounted(() => {
  // 从路由参数中读取搜索关键词
  const searchParam = route.query.search;
  if (searchParam && typeof searchParam === 'string') {
    searchQuery.value = searchParam;
  }
  loadInstances();
});
</script>

<template>
  <v-container fluid class="page-container pa-4">
    <!-- 页面标题 -->
    <div class="d-flex align-center justify-space-between mb-5">
      <div class="d-flex align-center">
        <v-avatar size="48" color="primary-container" class="mr-3">
          <v-icon size="24" color="on-primary-container">mdi-folder-multiple-outline</v-icon>
        </v-avatar>
        <div>
          <h1 class="text-h6 font-weight-bold">实例管理</h1>
          <p class="text-body-2 text-on-surface-variant mb-0">管理和启动你的游戏实例</p>
        </div>
      </div>
      <div class="d-flex align-center ga-2">
        <v-btn
          icon
          variant="tonal"
          color="secondary"
          size="small"
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

    <!-- 搜索和筛选工具栏 -->
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

        <v-select
          v-model="sortBy"
          :items="sortOptions"
          hide-details
          density="comfortable"
          variant="outlined"
          class="ml-3"
          style="max-width: 180px;"
        >
          <template #prepend-inner>
            <v-icon size="18" color="on-surface-variant">mdi-sort</v-icon>
          </template>
        </v-select>
        
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
    </div>

    <!-- 加载状态 - 骨架屏 -->
    <template v-if="loading">
      <v-row dense>
        <v-col
          v-for="i in 6"
          :key="i"
          cols="12"
          sm="6"
          md="4"
        >
          <v-card color="surface-container" variant="flat" class="skeleton-card">
            <v-card-text class="pa-4">
              <div class="d-flex align-center mb-3">
                <v-skeleton-loader type="avatar" class="mr-3" style="width: 44px; height: 44px;" />
                <div class="flex-grow-1">
                  <v-skeleton-loader type="text" style="width: 60%;" />
                  <v-skeleton-loader type="text" style="width: 40%;" class="mt-1" />
                </div>
              </div>
              <v-skeleton-loader type="text" style="width: 50%;" class="mb-3" />
              <v-skeleton-loader type="button" style="width: 100%;" />
            </v-card-text>
          </v-card>
        </v-col>
      </v-row>
    </template>

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
          :launching="launchingInstanceId === instance.id"
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
            <v-avatar size="44" :color="getLoaderColor(instance.loaderType).bgColor" class="mr-3">
              <v-icon size="22" :color="getLoaderColor(instance.loaderType).color">{{ getLoaderIcon(instance.loaderType) }}</v-icon>
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
              :loading="launchingInstanceId === instance.id"
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
    <v-dialog v-model="deleteDialog" max-width="380">
      <v-card color="surface-container-high">
        <v-card-text class="pa-5">
          <div class="text-h6 font-weight-bold mb-2">删除实例</div>
          <div class="text-body-2 text-on-surface-variant mb-4">
            确定要删除 <strong class="text-on-surface">{{ currentInstance?.name }}</strong> 吗？此操作无法撤销。
          </div>
          <v-text-field
            v-model="deleteConfirmName"
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
