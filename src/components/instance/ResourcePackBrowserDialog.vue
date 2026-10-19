<script setup lang="ts">
/**
 * 资源包浏览与安装对话框
 * 只按当前实例的 MC 版本搜索；资源包没有加载器维度，装好后仍要在面板里手动启用
 */
import { computed, ref, watch } from 'vue';
import { resourcePackApi } from '../../services';
import { useNotificationStore } from '../../stores/notificationStore';
import { useDebounceFn } from '../../composables/useDebounce';
import { getErrorMessage } from '../../utils/format';
import { logError } from '../../utils/logger';
import type { ModrinthModpack } from '../../types/events';

const props = defineProps<{
  modelValue: boolean;
  instanceName: string;
  gameVersion?: string | null;
}>();

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void;
  (e: 'installed'): void;
}>();

const notificationStore = useNotificationStore();

const query = ref('');
const results = ref<ModrinthModpack[]>([]);
const loading = ref(false);
const error = ref('');
const page = ref(1);
const totalHits = ref(0);
const installing = ref<string | null>(null);
const installedSlugs = ref<Set<string>>(new Set());
const PAGE_SIZE = 10;

const totalPages = computed(() => Math.ceil(totalHits.value / PAGE_SIZE));

async function search() {
  if (!props.modelValue) return;

  loading.value = true;
  error.value = '';
  try {
    const result = await resourcePackApi.searchModrinthResourcePacks(props.instanceName, {
      query: query.value.trim() || undefined,
      limit: PAGE_SIZE,
      offset: (page.value - 1) * PAGE_SIZE,
      sortBy: 'relevance',
    });
    results.value = result.hits || [];
    totalHits.value = result.total_hits || 0;
  } catch (e) {
    logError('搜索资源包失败', e, 'ResourcePackBrowserDialog');
    error.value = getErrorMessage(e);
    results.value = [];
    totalHits.value = 0;
  } finally {
    loading.value = false;
  }
}

const searchDebounced = useDebounceFn(() => {
  page.value = 1;
  void search();
}, 300);

watch(query, () => searchDebounced.call());

watch(page, () => {
  void search();
});

watch(
  () => props.modelValue,
  (open) => {
    if (open) {
      results.value = [];
      installedSlugs.value = new Set();
      page.value = 1;
      void search();
    }
  },
);

async function install(pack: ModrinthModpack) {
  if (installing.value) return;

  installing.value = pack.slug;
  try {
    const installed = await resourcePackApi.installModrinthResourcePack(props.instanceName, pack.slug);
    notificationStore.success('安装成功', `${installed.name} 已放入 resourcepacks`);
    installedSlugs.value = new Set([...installedSlugs.value, pack.slug]);
    emit('installed');
  } catch (e) {
    logError('安装资源包失败', e, 'ResourcePackBrowserDialog');
    notificationStore.error('安装失败', getErrorMessage(e), true);
  } finally {
    installing.value = null;
  }
}

function formatNumber(value: number): string {
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
  if (value >= 1_000) return `${(value / 1_000).toFixed(1)}K`;
  return String(value);
}

const ALLOWED_IMAGE_HOSTS = new Set(['cdn.modrinth.com', 'assets.modrinth.com']);

function iconUrl(url: string | undefined): string {
  if (!url) return '';
  try {
    const parsed = new URL(url);
    if (parsed.protocol !== 'https:') return '';
    return ALLOWED_IMAGE_HOSTS.has(parsed.hostname) ? url : '';
  } catch {
    return '';
  }
}
</script>

<template>
  <v-dialog
    :model-value="modelValue"
    max-width="720"
    scrollable
    @update:model-value="emit('update:modelValue', $event)"
  >
    <v-card color="surface-container-high">
      <v-card-title class="d-flex align-center pa-4 pb-2">
        <v-icon size="20" class="mr-2" color="on-surface-variant">mdi-texture-box</v-icon>
        <div class="flex-grow-1">
          <div class="text-title-medium font-weight-bold">安装资源包</div>
          <div class="text-body-small text-on-surface-variant">
            {{ instanceName }}
            <template v-if="gameVersion"> · {{ gameVersion }}</template>
          </div>
        </div>
        <v-btn icon size="small" variant="text" aria-label="关闭" @click="emit('update:modelValue', false)">
          <v-icon size="18">mdi-close</v-icon>
        </v-btn>
      </v-card-title>

      <v-card-text class="pa-4 pb-2">
        <v-text-field
          v-model="query"
          density="comfortable"
          variant="outlined"
          label="搜索资源包"
          placeholder="输入名称，例如 Faithful"
          hide-details
          clearable
        >
          <template #prepend-inner>
            <v-icon size="20" color="on-surface-variant">mdi-magnify</v-icon>
          </template>
        </v-text-field>
      </v-card-text>

      <v-divider />

      <v-card-text class="pa-0" style="min-height: 200px">
        <div v-if="loading" class="d-flex align-center justify-center pa-10">
          <v-progress-circular indeterminate size="28" color="primary" />
        </div>

        <div v-else-if="error" class="text-center pa-8">
          <div class="text-body-medium text-error mb-3">{{ error }}</div>
          <v-btn size="small" variant="tonal" color="primary" @click="search">重试</v-btn>
        </div>

        <div v-else-if="results.length === 0" class="text-center pa-10 text-on-surface-variant">
          <v-icon size="36" class="mb-2">mdi-texture-box</v-icon>
          <div class="text-body-medium">没有找到适配当前版本的资源包</div>
        </div>

        <v-list v-else lines="two" density="compact">
          <v-list-item v-for="pack in results" :key="pack.slug">
            <template #prepend>
              <v-avatar size="40" rounded="lg" class="mr-3" color="secondary-container">
                <v-img v-if="iconUrl(pack.icon_url)" :src="iconUrl(pack.icon_url)" cover>
                  <template #error>
                    <v-icon size="20" color="on-secondary-container">mdi-texture-box</v-icon>
                  </template>
                </v-img>
                <v-icon v-else size="20" color="on-secondary-container">mdi-texture-box</v-icon>
              </v-avatar>
            </template>

            <v-list-item-title class="text-body-medium font-weight-medium">
              {{ pack.title }}
            </v-list-item-title>
            <v-list-item-subtitle class="text-body-small pack-desc">
              {{ pack.description }}
            </v-list-item-subtitle>

            <template #append>
              <div class="d-flex align-center ga-2">
                <span class="text-body-small text-on-surface-variant d-none d-sm-inline">
                  <v-icon size="14" class="mr-1">mdi-download</v-icon>{{ formatNumber(pack.downloads) }}
                </span>
                <v-btn
                  size="small"
                  variant="tonal"
                  color="primary"
                  :loading="installing === pack.slug"
                  :disabled="installing !== null || installedSlugs.has(pack.slug)"
                  @click="install(pack)"
                >
                  {{ installedSlugs.has(pack.slug) ? '已安装' : '安装' }}
                </v-btn>
              </div>
            </template>
          </v-list-item>
        </v-list>
      </v-card-text>

      <v-divider v-if="totalPages > 1" />
      <v-card-actions v-if="totalPages > 1" class="pa-2">
        <span class="text-body-small text-on-surface-variant ml-2">共 {{ totalHits }} 个资源包</span>
        <v-spacer />
        <v-pagination v-model="page" :length="totalPages" :total-visible="5" density="compact" color="primary" />
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<style scoped>
.pack-desc {
  display: -webkit-box;
  -webkit-line-clamp: 1;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
</style>
