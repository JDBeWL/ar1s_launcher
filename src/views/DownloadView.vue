<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useDownloadStore } from '@/stores/downloadStore';
import { useSettingsStore } from '@/stores/settings';
import { useNotificationStore } from '@/stores/notificationStore';
import type { MinecraftVersion, VersionManifest } from '@/types/events';

// SVG图标
const grassBlockIcon = '/icons/grass_block.svg';
const dirtPathIcon = '/icons/dirt_path.svg';

// 使用全局下载状态
const downloadStore = useDownloadStore();
const settingsStore = useSettingsStore();
const notificationStore = useNotificationStore();

// --- State ---
const allVersions = ref<MinecraftVersion[]>([]);
const loading = ref(false);
const searchQuery = ref('');
const versionType = ref('release');
const sortOrder = ref('newest');
const itemsPerPage = 10;
const currentPage = ref(1);

const typeMeta: Record<string, { label: string; color: string; icon: string }> = {
  release: { label: '正式版', color: 'success', icon: grassBlockIcon },
  snapshot: { label: '快照版', color: 'warning', icon: dirtPathIcon }
};

const getTypeMeta = (type: string) => typeMeta[type] ?? { label: '其他版本', color: 'primary', icon: 'mdi-cube-outline' };
const getTypeLabel = (type: string) => getTypeMeta(type).label;
const getTypeColor = (type: string) => getTypeMeta(type).color;
const getTypeIcon = (type: string) => getTypeMeta(type).icon;

const formatDateTime = (value: string) => {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return '发布日期未知';
  }
  return date.toLocaleString();
};

// 从store获取下载状态
const isDownloading = computed(() => downloadStore.isDownloading);
const selectedVersion = computed(() => downloadStore.selectedVersion);

// --- Methods ---

// Fetch versions from backend
async function fetchVersions() {
  try {
    loading.value = true;
    const result = await invoke<VersionManifest>('get_versions');
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

// Start a download
async function startDownload(versionId: string) {
  await downloadStore.startDownload(versionId, settingsStore.downloadMirror);
}

// Cancel a download
async function cancelDownload() {
  await downloadStore.cancelDownload();
}

// --- Computed Properties ---

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
  } else if (sortOrder.value === 'az') {
    filtered.sort((a, b) => a.id.localeCompare(b.id));
  } else if (sortOrder.value === 'za') {
    filtered.sort((a, b) => b.id.localeCompare(a.id));
  }
  
  return filtered;
});

const paginatedVersions = computed(() => {
  const start = (currentPage.value - 1) * itemsPerPage;
  const end = start + itemsPerPage;
  return filteredVersions.value.slice(start, end);
});

const totalPages = computed(() => {
  return Math.ceil(filteredVersions.value.length / itemsPerPage);
});


// --- Lifecycle Hooks ---

onMounted(async () => {
  await settingsStore.loadDownloadMirror();
  await fetchVersions();
});
</script>

<template>
  <v-container>
    <v-card>
      <v-card-title class="d-flex align-center">
        下载 Minecraft
        <v-spacer></v-spacer>
        <v-btn variant="text" icon="mdi-refresh" @click="fetchVersions" :disabled="isDownloading"></v-btn>
      </v-card-title>
      <v-card-text>
        <!-- Search and Filter -->
        <v-row no-gutters class="align-center">
          <v-col class="flex-grow-1 pr-2">
            <v-text-field
              v-model="searchQuery"
              label="搜索版本"
              prepend-inner-icon="mdi-magnify"
              clearable
              hide-details
              :disabled="isDownloading"
              @update:model-value="currentPage = 1"
            ></v-text-field>
          </v-col>
          <v-col class="shrink pr-2" style="max-width: 150px;">
            <v-select
              v-model="versionType"
              label="版本类型"
              :items="[
                { title: '全部', value: 'all' },
                { title: '正式版', value: 'release' },
                { title: '快照版', value: 'snapshot' }
              ]"
              hide-details
              :disabled="isDownloading"
              @update:model-value="currentPage = 1"
            ></v-select>
          </v-col>
          <v-col class="shrink" style="max-width: 180px;">
            <v-select
              v-model="sortOrder"
              label="排序方式"
              :items="[
                { title: '最新优先', value: 'newest' },
                { title: '最旧优先', value: 'oldest' }
              ]"
              hide-details
              :disabled="isDownloading"
            ></v-select>
          </v-col>
        </v-row>
        
        <!-- Download Source / Progress Bar -->
        <v-row class="mt-4" :style="{ minHeight: '5px' }">
          <v-col v-if="isDownloading">
            <v-card-subtitle>下载任务已开始</v-card-subtitle>
            <v-btn
              variant="text"
              color="primary"
              @click="downloadStore.showDownloadNotification()"
            >
              查看下载进度
            </v-btn>
          </v-col>
        </v-row>

        <!-- Versions List -->
        <v-row>
          <v-col cols="12">
            <div v-if="loading" class="text-center py-6">
              正在加载...
            </div>
            <template v-else>
              <div v-if="paginatedVersions.length === 0" class="text-center py-6">
                没有找到匹配的版本
              </div>
              <template v-else>
                <v-card
                  v-for="item in paginatedVersions"
                  :key="item.id"
                  class="version-card mb-3"
                  variant="outlined"
                >
                  <div class="version-card__left">
                    <img :src="getTypeIcon(item.type)" :alt="getTypeLabel(item.type)" width="48" height="48" class="mr-3">
                    <div class="version-card__info">
                      <div class="version-card__title">
                        <span class="version-card__id">{{ item.id }}</span>
                        <v-chip
                          size="small"
                          :color="getTypeColor(item.type)"
                          variant="tonal"
                          class="font-weight-medium"
                        >
                          {{ getTypeLabel(item.type) }}
                        </v-chip>
                      </div>
                      <div class="version-card__date">
                        {{ formatDateTime(item.releaseTime) }}
                      </div>
                    </div>
                  </div>
                  <div class="version-card__right">
                    <v-btn
                      v-if="isDownloading && selectedVersion === item.id"
                      color="error"
                      variant="tonal"
                      size="small"
                      icon="mdi-close"
                      @click="cancelDownload"
                    ></v-btn>
                    <v-btn
                      v-else
                      color="primary"
                      variant="tonal"
                      size="small"
                      icon="mdi-download"
                      :disabled="isDownloading"
                      @click="startDownload(item.id)"
                    ></v-btn>
                  </div>
                </v-card>
              </template>
            </template>

            <v-pagination
              v-if="totalPages > 1"
              v-model="currentPage"
              :length="totalPages"
              :disabled="isDownloading"
              :total-visible="7"
              class="mt-4"
            ></v-pagination>
          </v-col>
        </v-row>
      </v-card-text>
    </v-card>
  </v-container>
</template>

<style scoped>
.version-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px;
  gap: 12px;
  min-height: 64px;
}

.version-card__left {
  display: flex;
  align-items: center;
  flex: 1;
  min-width: 0;
  gap: 12px;
}

.version-card__info {
  display: flex;
  flex-direction: column;
  min-width: 0;
  gap: 6px;
}

.version-card__title {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.version-card__id {
  font-weight: 600;
  font-size: 1.1rem;
  word-break: break-word;
}

.version-card__date {
  font-size: 0.875rem;
  opacity: 0.7;
}

.version-card__right {
  display: flex;
  align-items: center;
  gap: 8px;
}

@media (max-width: 600px) {
  .version-card {
    flex-direction: column;
    align-items: flex-start;
  }

  .version-card__right {
    width: 100%;
    justify-content: flex-end;
  }
}
</style>