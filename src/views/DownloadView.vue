<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useDownloadStore } from '@/stores/downloadStore';
import { useSettingsStore } from '@/stores/settings';
import { useNotificationStore } from '@/stores/notificationStore';
import type { MinecraftVersion, VersionManifest } from '@/types/events';

const downloadStore = useDownloadStore();
const settingsStore = useSettingsStore();
const notificationStore = useNotificationStore();

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

const isDownloading = computed(() => downloadStore.isDownloading);
const selectedVersion = computed(() => downloadStore.selectedVersion);

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

async function startDownload(versionId: string) {
  await downloadStore.startDownload(versionId, settingsStore.downloadMirror);
}

async function cancelDownload() {
  await downloadStore.cancelDownload();
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
  await settingsStore.loadDownloadMirror();
  await fetchVersions();
});
</script>

<template>
  <v-container fluid class="download-container pa-4">
    <!-- 页面标题 -->
    <div class="d-flex align-center justify-space-between mb-4">
      <div class="d-flex align-center">
        <v-avatar size="40" class="mr-3 avatar-outlined">
          <v-icon size="20">mdi-download</v-icon>
        </v-avatar>
        <div>
          <h1 class="text-h6 font-weight-bold">下载 Minecraft</h1>
          <p class="text-body-2 text-medium-emphasis mb-0">选择并下载游戏版本</p>
        </div>
      </div>
      <v-btn
        icon
        variant="outlined"
        size="small"
        :loading="loading"
        :disabled="isDownloading"
        @click="fetchVersions"
      >
        <v-icon size="18">mdi-refresh</v-icon>
      </v-btn>
    </div>

    <!-- 下载进度提示 -->
    <v-alert
      v-if="isDownloading"
      variant="outlined"
      rounded="lg"
      density="compact"
      class="mb-4"
    >
      <template #prepend>
        <v-progress-circular indeterminate size="20" width="2" class="mr-2" />
      </template>
      <div class="d-flex align-center justify-space-between">
        <span class="text-body-2">正在下载 {{ selectedVersion }}...</span>
        <v-btn
          variant="text"
          size="small"
          @click="downloadStore.showDownloadNotification()"
        >
          查看进度
        </v-btn>
      </div>
    </v-alert>

    <!-- 搜索和筛选 -->
    <v-card variant="outlined" rounded="lg" class="mb-4">
      <v-card-text class="pa-3">
        <v-row dense align="center">
          <v-col cols="12" sm="5">
            <v-text-field
              v-model="searchQuery"
              placeholder="搜索版本号..."
              variant="outlined"
              density="compact"
              rounded="lg"
              hide-details
              clearable
              :disabled="isDownloading"
              @update:model-value="currentPage = 1"
            >
              <template #prepend-inner>
                <v-icon size="18">mdi-magnify</v-icon>
              </template>
            </v-text-field>
          </v-col>
          <v-col cols="6" sm="4">
            <v-btn-toggle
              v-model="versionType"
              mandatory
              rounded="lg"
              density="compact"
              variant="outlined"
              divided
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
              variant="outlined"
              density="compact"
              rounded="lg"
              hide-details
              :disabled="isDownloading"
            >
              <template #prepend-inner>
                <v-icon size="18">mdi-sort</v-icon>
              </template>
            </v-select>
          </v-col>
        </v-row>
      </v-card-text>
    </v-card>

    <!-- 版本列表 -->
    <div v-if="loading" class="text-center py-12">
      <v-progress-circular indeterminate size="40" />
      <div class="text-body-2 text-medium-emphasis mt-3">加载版本列表...</div>
    </div>

    <div v-else-if="paginatedVersions.length === 0" class="text-center py-12">
      <v-avatar size="64" class="mb-3 avatar-outlined">
        <v-icon size="32">mdi-magnify-close</v-icon>
      </v-avatar>
      <div class="text-body-1 font-weight-medium">没有找到匹配的版本</div>
      <div class="text-body-2 text-medium-emphasis">尝试调整搜索条件</div>
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
            variant="outlined"
            rounded="lg"
            class="version-card h-100"
            :class="{ 'version-card--downloading': isDownloading && selectedVersion === item.id }"
          >
            <v-card-text class="pa-3">
              <div class="d-flex align-center justify-space-between mb-2">
                <div class="d-flex align-center">
                  <v-avatar
                    size="36"
                    class="mr-2"
                    :class="item.type === 'release' ? 'avatar-release' : 'avatar-snapshot'"
                  >
                    <v-icon size="18">
                      {{ item.type === 'release' ? 'mdi-check-decagram' : 'mdi-flask' }}
                    </v-icon>
                  </v-avatar>
                  <div>
                    <div class="text-subtitle-2 font-weight-bold">{{ item.id }}</div>
                    <div class="text-caption text-medium-emphasis">{{ formatDateTime(item.releaseTime) }}</div>
                  </div>
                </div>
                <v-chip
                  size="x-small"
                  :variant="item.type === 'release' ? 'flat' : 'outlined'"
                  :class="item.type === 'release' ? '' : 'text-medium-emphasis'"
                >
                  {{ item.type === 'release' ? '正式版' : '快照' }}
                </v-chip>
              </div>

              <v-btn
                v-if="isDownloading && selectedVersion === item.id"
                variant="outlined"
                block
                size="small"
                rounded="lg"
                @click="cancelDownload"
              >
                <v-icon start size="16">mdi-close</v-icon>
                取消下载
              </v-btn>
              <v-btn
                v-else
                variant="outlined"
                block
                size="small"
                rounded="lg"
                :disabled="isDownloading"
                @click="startDownload(item.id)"
              >
                <v-icon start size="16">mdi-download</v-icon>
                下载
              </v-btn>
            </v-card-text>
          </v-card>
        </v-col>
      </v-row>

      <!-- 分页 -->
      <div v-if="totalPages > 1" class="d-flex justify-center mt-4">
        <v-pagination
          v-model="currentPage"
          :length="totalPages"
          :disabled="isDownloading"
          :total-visible="5"
          density="compact"
          rounded="lg"
        />
      </div>

      <!-- 统计信息 -->
      <div class="text-center text-caption text-medium-emphasis mt-3">
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

.avatar-outlined {
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
}

.avatar-release {
  background: rgba(var(--v-theme-on-surface), 0.08);
}

.avatar-snapshot {
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
}

.version-card {
  transition: transform 0.2s ease, border-color 0.2s ease;
}

.version-card:hover {
  transform: translateY(-2px);
}

.version-card--downloading {
  border-color: rgb(var(--v-theme-primary));
}
</style>
