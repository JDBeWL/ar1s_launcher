<script setup lang="ts">
/**
 * 光影包浏览与安装对话框
 * 只按当前实例的 MC 版本搜索；Modrinth 上光影包挂在 iris / optifine 两个 loader 下，
 * 后端把这两个条件按「任一命中」处理。装好后默认顺手启用，省一次点击。
 */
import { computed, ref, watch } from 'vue';
import { shaderPackApi } from '../../services';
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
// 装完直接写进游戏配置，否则玩家还得回面板再点一次「启用」
const activateAfterInstall = ref(true);
const PAGE_SIZE = 10;

const totalPages = computed(() => Math.ceil(totalHits.value / PAGE_SIZE));

async function search() {
  if (!props.modelValue) return;

  loading.value = true;
  error.value = '';
  try {
    const result = await shaderPackApi.searchModrinthShaderPacks(props.instanceName, {
      query: query.value.trim() || undefined,
      limit: PAGE_SIZE,
      offset: (page.value - 1) * PAGE_SIZE,
      sortBy: 'relevance',
    });
    results.value = result.hits || [];
    totalHits.value = result.total_hits || 0;
  } catch (e) {
    logError('搜索光影包失败', e, 'ShaderPackBrowserDialog');
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
    const installed = await shaderPackApi.installModrinthShaderPack(props.instanceName, pack.slug);
    installedSlugs.value = new Set([...installedSlugs.value, pack.slug]);

    if (activateAfterInstall.value) {
      try {
        await shaderPackApi.selectShaderPack(props.instanceName, installed.fileName);
        notificationStore.success('安装并启用', `${installed.name}（${installed.versionNumber}）`);
      } catch (e) {
        // 包已经装好，只是没能写进游戏配置，别把整个安装判成失败
        logError('安装后启用光影失败', e, 'ShaderPackBrowserDialog');
        notificationStore.warning('已安装但未启用', getErrorMessage(e));
      }
    } else {
      notificationStore.success('安装成功', `${installed.name} 已放入 shaderpacks`);
    }

    emit('installed');
  } catch (e) {
    logError('安装光影包失败', e, 'ShaderPackBrowserDialog');
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
        <v-icon size="20" class="mr-2" color="on-surface-variant">mdi-shimmer</v-icon>
        <div class="flex-grow-1">
          <div class="text-title-medium font-weight-bold">安装光影包</div>
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
          label="搜索光影包"
          placeholder="输入名称，例如 Complementary"
          hide-details
          clearable
        >
          <template #prepend-inner>
            <v-icon size="20" color="on-surface-variant">mdi-magnify</v-icon>
          </template>
        </v-text-field>

        <v-switch
          v-model="activateAfterInstall"
          label="安装后立即启用"
          hide-details
          inset
          density="compact"
          color="primary"
          class="mt-2"
        />
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
          <v-icon size="36" class="mb-2">mdi-shimmer</v-icon>
          <div class="text-body-medium">没有找到适配当前版本的光影包</div>
        </div>

        <v-list v-else lines="two" density="compact">
          <v-list-item v-for="pack in results" :key="pack.slug">
            <template #prepend>
              <v-avatar size="40" rounded="lg" class="mr-3" color="secondary-container">
                <v-img v-if="iconUrl(pack.icon_url)" :src="iconUrl(pack.icon_url)" cover>
                  <template #error>
                    <v-icon size="20" color="on-secondary-container">mdi-shimmer</v-icon>
                  </template>
                </v-img>
                <v-icon v-else size="20" color="on-secondary-container">mdi-shimmer</v-icon>
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
        <span class="text-body-small text-on-surface-variant ml-2">共 {{ totalHits }} 个光影包</span>
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
