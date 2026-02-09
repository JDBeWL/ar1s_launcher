<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { useDownloadStore } from '@/stores/downloadStore';
import { useSettingsStore } from '@/stores/settings';
import { useNotificationStore } from '@/stores/notificationStore';
import { versionApi } from '@/services';
import type { MinecraftVersion } from '@/types/events';
import { useVersionManager } from '@/composables/useVersionManager';

const downloadStore = useDownloadStore();
const settingsStore = useSettingsStore();
const notificationStore = useNotificationStore();
const { installedVersions, loadGameDir, initListeners } = useVersionManager();

const allVersions = ref<MinecraftVersion[]>([]);
const loading = ref(false);
const searchQuery = ref('');
const versionType = ref('release');
const sortOrder = ref('newest');
const itemsPerPage = 12;
const currentPage = ref(1);

const formatDateTime = (value: string) => {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return '发布日期未知';
  return date.toLocaleDateString();
};

// 根据版本类型获取对应图标
const getVersionIcon = (type: string) => {
  switch (type) {
    case 'release':
      return '/icons/grass_block.svg';
    case 'snapshot':
      return '/icons/dirt_path.svg';
    default:
      return '/icons/snow_grass.svg';
  }
};

// 获取版本类型名称
const getVersionTypeName = (type: string) => {
  switch (type) {
    case 'release':
      return '正式版';
    case 'snapshot':
      return '快照';
    case 'old_alpha':
      return 'Alpha';
    case 'old_beta':
      return 'Beta';
    default:
      return '其他';
  }
};

// 获取版本标签颜色
const getVersionChipColor = (type: string) => {
  switch (type) {
    case 'release':
      return 'primary';
    case 'snapshot':
      return 'warning';
    default:
      return 'secondary';
  }
};

const isDownloading = computed(() => downloadStore.isDownloading);
const installedSet = computed(() => new Set(installedVersions.value));

function isInstalled(versionId: string) {
  return installedSet.value.has(versionId);
}

async function fetchVersions() {
  try {
    loading.value = true;
    const result = await versionApi.getVersions();
    if (result?.versions) {
      allVersions.value = result.versions;
    } else {
      allVersions.value = [];
    }
  } catch (err) {
    console.error('获取版本列表失败:', err);
    notificationStore.error('获取版本列表失败', '请检查网络连接或稍后再试');
  } finally {
    loading.value = false;
  }
}

async function startDownload(versionId: string) {
  await downloadStore.startDownload(versionId, settingsStore.downloadMirror);
}

const filteredVersions = computed(() => {
  let filtered = allVersions.value.filter(version => {
    const matchesSearch = searchQuery.value === '' || version.id.toLowerCase().includes(searchQuery.value.toLowerCase());
    const matchesType = versionType.value === 'all' || version.type === versionType.value;
    return matchesSearch && matchesType;
  });

  if (sortOrder.value === 'newest') {
    filtered.sort((a, b) => new Date(b.releaseTime).getTime() - new Date(a.releaseTime).getTime());
  } else if (sortOrder.value === 'oldest') {
    filtered.sort((a, b) => new Date(a.releaseTime).getTime() - new Date(b.releaseTime).getTime());
  }
  
  return filtered;
});

const paginatedVersions = computed(() => {
  const start = (currentPage.value - 1) * itemsPerPage;
  return filteredVersions.value.slice(start, start + itemsPerPage);
});

const totalPages = computed(() => Math.ceil(filteredVersions.value.length / itemsPerPage));

onMounted(async () => {
  await Promise.all([
    settingsStore.loadDownloadMirror(),
    fetchVersions(),
    loadGameDir(),
    initListeners()
  ]);
});
</script>

<template>
  <v-container fluid class="download-container pa-4">
    <!-- 页面标题 -->
    <div class="d-flex align-center justify-space-between mb-5">
      <div class="d-flex align-center">
        <v-avatar size="48" color="primary-container" class="mr-3">
          <v-icon size="24" color="on-primary-container">mdi-download</v-icon>
        </v-avatar>
        <div>
          <h1 class="text-h6 font-weight-bold">下载 Minecraft</h1>
          <p class="text-body-2 text-on-surface-variant mb-0">选择并下载游戏版本</p>
        </div>
      </div>
      <v-btn
        icon
        variant="tonal"
        color="secondary"
        size="small"
        :loading="loading"
        :disabled="isDownloading"
        @click="fetchVersions"
      >
        <v-icon size="20">mdi-refresh</v-icon>
      </v-btn>
    </div>

    <!-- 搜索和筛选 -->
    <v-card color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <v-row dense align="center">
          <v-col cols="12" sm="5">
            <v-text-field
              v-model="searchQuery"
              placeholder="搜索版本号..."
              hide-details
              clearable
              :disabled="isDownloading"
              @update:model-value="currentPage = 1"
            >
              <template #prepend-inner>
                <v-icon size="20" color="on-surface-variant">mdi-magnify</v-icon>
              </template>
            </v-text-field>
          </v-col>
          <v-col cols="6" sm="4">
            <v-btn-toggle
              v-model="versionType"
              mandatory
              density="comfortable"
              divided
              color="primary"
              :disabled="isDownloading"
              @update:model-value="currentPage = 1"
            >
              <v-btn value="release" size="small">正式版</v-btn>
              <v-btn value="snapshot" size="small">快照</v-btn>
              <v-btn value="all" size="small">全部</v-btn>
            </v-btn-toggle>
          </v-col>
          <v-col cols="6" sm="3">
            <v-select
              v-model="sortOrder"
              :items="[
                { title: '最新优先', value: 'newest' },
                { title: '最旧优先', value: 'oldest' }
              ]"
              hide-details
              :disabled="isDownloading"
            >
              <template #prepend-inner>
                <v-icon size="20" color="on-surface-variant">mdi-sort</v-icon>
              </template>
            </v-select>
          </v-col>
        </v-row>
      </v-card-text>
    </v-card>

    <!-- 版本列表 - 骨架屏加载 -->
    <template v-if="loading">
      <v-row dense>
        <v-col
          v-for="i in 12"
          :key="i"
          cols="12"
          sm="6"
          md="4"
        >
          <v-card color="surface-container" class="version-card">
            <v-card-text class="pa-4">
              <div class="d-flex align-center justify-space-between mb-3">
                <div class="d-flex align-center">
                  <v-skeleton-loader type="avatar" class="mr-3" style="width: 40px; height: 40px;" />
                  <div>
                    <v-skeleton-loader type="text" style="width: 80px;" />
                    <v-skeleton-loader type="text" style="width: 60px;" class="mt-1" />
                  </div>
                </div>
                <v-skeleton-loader type="chip" style="width: 50px;" />
              </div>
              <v-skeleton-loader type="button" style="width: 100%;" />
            </v-card-text>
          </v-card>
        </v-col>
      </v-row>
    </template>

    <div v-else-if="paginatedVersions.length === 0" class="text-center py-12">
      <v-avatar size="80" color="surface-container-high" class="mb-4">
        <v-icon size="40" color="on-surface-variant">mdi-magnify-close</v-icon>
      </v-avatar>
      <div class="text-body-1 font-weight-medium">没有找到匹配的版本</div>
      <div class="text-body-2 text-on-surface-variant">尝试调整搜索条件</div>
    </div>

    <template v-else>
      <!-- 版本网格 -->
      <v-row dense>
        <v-col
          v-for="item in paginatedVersions"
          :key="item.id"
          cols="12"
          sm="6"
          md="4"
        >
          <v-card
            color="surface-container"
            class="version-card h-100"
          >
            <v-card-text class="pa-4">
              <div class="d-flex align-center justify-space-between mb-3">
                <div class="d-flex align-center">
                  <v-avatar size="40" class="mr-3 version-icon-avatar">
                    <img 
                      :src="getVersionIcon(item.type)" 
                      :alt="getVersionTypeName(item.type)"
                      class="version-icon"
                    />
                  </v-avatar>
                  <div>
                    <div class="text-subtitle-2 font-weight-bold">{{ item.id }}</div>
                    <div class="text-caption text-on-surface-variant">{{ formatDateTime(item.releaseTime) }}</div>
                  </div>
                </div>
                <div class="d-flex align-center ga-2">
                  <v-chip
                    size="small"
                    :color="getVersionChipColor(item.type)"
                    variant="tonal"
                  >
                    {{ getVersionTypeName(item.type) }}
                  </v-chip>
                  <v-chip
                    v-if="isInstalled(item.id)"
                    size="small"
                    color="success"
                    variant="tonal"
                  >
                    已安装
                  </v-chip>
                </div>
              </div>

              <v-btn
                variant="tonal"
                color="primary"
                block
                size="small"
                :disabled="isDownloading || isInstalled(item.id)"
                @click="startDownload(item.id)"
              >
                <v-icon start size="18">
                  {{ isInstalled(item.id) ? 'mdi-check' : 'mdi-download' }}
                </v-icon>
                {{ isInstalled(item.id) ? '已安装' : '下载' }}
              </v-btn>
            </v-card-text>
          </v-card>
        </v-col>
      </v-row>

      <!-- 分页 -->
      <div v-if="totalPages > 1" class="d-flex justify-center mt-5">
        <v-pagination
          v-model="currentPage"
          :length="totalPages"
          :disabled="isDownloading"
          :total-visible="5"
          density="comfortable"
          color="primary"
        />
      </div>

      <!-- 统计信息 -->
      <div class="text-center text-caption text-on-surface-variant mt-3">
        共 {{ filteredVersions.length }} 个版本
      </div>
    </template>
  </v-container>
</template>

<style scoped>
.download-container {
  max-width: 900px;
  margin: 0 auto;
}

.version-icon-avatar {
  background: transparent;
}

.version-icon {
  width: 32px;
  height: 32px;
  object-fit: contain;
}
</style>
