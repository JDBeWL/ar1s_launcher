<script setup lang="ts">
import { ref, onMounted, computed, onUnmounted, watch, nextTick } from 'vue';
import { useDownloadStore } from '@/stores/downloadStore';
import { useSettingsStore } from '@/stores/settings';
import { useNotificationStore } from '@/stores/notificationStore';
import { versionApi } from '@/services';
import type { MinecraftVersion } from '@/types/events';
import { useVersionManager } from '@/composables/useVersionManager';
import { logError } from '@/utils/logger';
import { sortVersionsByReleaseTime } from '@/utils/format';

const downloadStore = useDownloadStore();
const settingsStore = useSettingsStore();
const notificationStore = useNotificationStore();
const { installedVersions, loadGameDir, initListeners } = useVersionManager();

const allVersions = ref<MinecraftVersion[]>([]);
const loading = ref(false);
const loadError = ref(false);
const searchQuery = ref('');
const versionType = ref('release');
const sortOrder = ref('newest');
const batchSize = 24;
const displayedCount = ref(batchSize);
const sentinelRef = ref<HTMLElement | null>(null);
let observer: IntersectionObserver | null = null;

const formatDateTime = (value: string) => {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return '发布日期未知';
  return date.toLocaleDateString();
};

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
  const isRefresh = allVersions.value.length > 0;
  try {
    if (!isRefresh) loading.value = true;
    const result = await versionApi.getVersions();
    if (result?.versions) {
      allVersions.value = result.versions;
    } else {
      allVersions.value = [];
    }
    loadError.value = false;
  } catch (err) {
    logError('获取版本列表失败', err, 'DownloadView');
    notificationStore.error('获取版本列表失败', '请检查网络连接或稍后再试');
    loadError.value = true;
  } finally {
    loading.value = false;
  }
}

async function startDownload(versionId: string) {
  await downloadStore.startDownload(versionId, settingsStore.downloadMirror);
}

const filteredVersions = computed(() => {
  let filtered = allVersions.value.filter((version) => {
    const matchesSearch =
      searchQuery.value === '' || version.id.toLowerCase().includes(searchQuery.value.toLowerCase());
    const matchesType = versionType.value === 'all' || version.type === versionType.value;
    return matchesSearch && matchesType;
  });

  if (sortOrder.value === 'newest' || sortOrder.value === 'oldest') {
    return sortVersionsByReleaseTime(filtered, sortOrder.value);
  }

  return filtered;
});

const displayedVersions = computed(() => {
  return filteredVersions.value.slice(0, displayedCount.value);
});

const hasMore = computed(() => displayedCount.value < filteredVersions.value.length);

function loadMore() {
  if (hasMore.value) {
    displayedCount.value += batchSize;
  }
}

function setupObserver() {
  if (observer) observer.disconnect();
  observer = new IntersectionObserver(
    (entries) => {
      if (entries[0]?.isIntersecting && hasMore.value) {
        loadMore();
      }
    },
    { rootMargin: '200px' },
  );
  if (sentinelRef.value) {
    observer.observe(sentinelRef.value);
  }
}

watch([searchQuery, versionType, sortOrder], () => {
  displayedCount.value = batchSize;
  nextTick(() => setupObserver());
});

onMounted(async () => {
  await Promise.all([settingsStore.loadSettings(), fetchVersions(), loadGameDir(), initListeners()]);
  nextTick(() => setupObserver());
});

onUnmounted(() => {
  if (observer) {
    observer.disconnect();
    observer = null;
  }
});
</script>

<template>
  <v-container fluid class="page-container pa-4">
    <v-card variant="flat" color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <v-row density="compact" align="center">
          <v-col cols="12" sm="5">
            <v-text-field
              density="comfortable"
              v-model="searchQuery"
              label="搜索版本号"
              hide-details
              clearable
              :disabled="isDownloading"
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
              border
              divided
              rounded="pill"
              density="comfortable"
              variant="text"
              color="primary"
              :disabled="isDownloading"
            >
              <v-btn value="release">正式版</v-btn>
              <v-btn value="snapshot">快照</v-btn>
              <v-btn value="all">全部</v-btn>
            </v-btn-toggle>
          </v-col>
          <v-col cols="6" sm="3">
            <div class="d-flex align-center ga-2">
              <v-select
                density="comfortable"
                v-model="sortOrder"
                aria-label="排序方式"
                :items="[
                  { title: '最新优先', value: 'newest' },
                  { title: '最旧优先', value: 'oldest' },
                ]"
                hide-details
                class="flex-grow-1"
                :disabled="isDownloading"
              >
                <template #prepend-inner>
                  <v-icon size="20" color="on-surface-variant">mdi-sort</v-icon>
                </template>
              </v-select>
              <v-btn
                icon
                variant="tonal"
                color="secondary"
                aria-label="重新获取版本列表"
                :loading="loading"
                :disabled="isDownloading"
                @click="fetchVersions"
              >
                <v-icon size="20">mdi-refresh</v-icon>
              </v-btn>
            </div>
          </v-col>
        </v-row>
      </v-card-text>
    </v-card>

    <template v-if="loading">
      <v-row density="compact">
        <v-col v-for="i in 12" :key="i" cols="12" sm="6" md="4">
          <v-card variant="flat" color="surface-container" class="version-card">
            <v-card-text class="pa-4">
              <div class="d-flex align-center justify-space-between mb-3">
                <div class="d-flex align-center">
                  <v-skeleton-loader type="avatar" class="mr-3" style="width: 40px; height: 40px" />
                  <div>
                    <v-skeleton-loader type="text" style="width: 80px" />
                    <v-skeleton-loader type="text" style="width: 60px" class="mt-1" />
                  </div>
                </div>
                <v-skeleton-loader type="chip" style="width: 50px" />
              </div>
              <v-skeleton-loader type="button" style="width: 100%" />
            </v-card-text>
          </v-card>
        </v-col>
      </v-row>
    </template>

    <div v-else-if="loadError" class="text-center py-12">
      <v-avatar size="80" color="surface-container-high" class="mb-4">
        <v-icon size="40" color="on-surface-variant">mdi-cloud-off-outline</v-icon>
      </v-avatar>
      <div class="text-body-large font-weight-medium">版本列表获取失败</div>
      <div class="text-body-medium text-on-surface-variant mb-4">检查网络连接后重试</div>
      <v-btn variant="tonal" color="primary" prepend-icon="mdi-refresh" @click="fetchVersions">重新加载</v-btn>
    </div>

    <div v-else-if="displayedVersions.length === 0" class="text-center py-12">
      <v-avatar size="80" color="surface-container-high" class="mb-4">
        <v-icon size="40" color="on-surface-variant">mdi-magnify-close</v-icon>
      </v-avatar>
      <div class="text-body-large font-weight-medium">没有找到匹配的版本</div>
      <div class="text-body-medium text-on-surface-variant">尝试调整搜索条件</div>
    </div>

    <template v-else>
      <v-row density="compact">
        <v-col v-for="item in displayedVersions" :key="item.id" cols="12" sm="6" md="4">
          <v-card variant="flat" color="surface-container" class="version-card h-100">
            <v-card-text class="pa-4">
              <div class="d-flex align-center justify-space-between mb-3">
                <div class="d-flex align-center">
                  <v-avatar size="40" class="mr-3 version-icon-avatar">
                    <img :src="getVersionIcon(item.type)" :alt="getVersionTypeName(item.type)" class="version-icon" />
                  </v-avatar>
                  <div>
                    <div class="text-title-small font-weight-bold">{{ item.id }}</div>
                    <div class="text-body-small text-on-surface-variant">{{ formatDateTime(item.releaseTime) }}</div>
                  </div>
                </div>
                <div class="d-flex align-center ga-2">
                  <v-chip size="small" :color="getVersionChipColor(item.type)" variant="tonal">
                    {{ getVersionTypeName(item.type) }}
                  </v-chip>
                </div>
              </div>

              <div v-if="isInstalled(item.id)" class="install-state text-body-medium font-weight-medium">
                <v-icon size="18" class="mr-2">mdi-check</v-icon>
                已安装
              </div>
              <v-btn
                v-else
                variant="tonal"
                color="primary"
                block
                size="small"
                :disabled="isDownloading"
                @click="startDownload(item.id)"
              >
                <v-icon start size="18">mdi-download</v-icon>
                下载
              </v-btn>
            </v-card-text>
          </v-card>
        </v-col>
      </v-row>

      <div ref="sentinelRef" class="infinite-scroll-sentinel" />

      <div class="text-center text-body-small text-on-surface-variant mt-3">
        {{
          hasMore
            ? `已显示 ${displayedVersions.length} / ${filteredVersions.length} 个版本`
            : `共 ${filteredVersions.length} 个版本`
        }}
      </div>
    </template>
  </v-container>
</template>

<style scoped>
.version-icon-avatar {
  background: transparent;
}

.version-icon {
  width: 32px;
  height: 32px;
  object-fit: contain;
}

.infinite-scroll-sentinel {
  height: 1px;
  width: 100%;
}
</style>
