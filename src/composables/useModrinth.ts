import { ref, computed, watch } from 'vue';
import { api } from '../services';
import { useDebounceFn } from './useDebounce';
import { logError } from '../utils/logger';
import { compareVersionDesc } from '../utils/format';
import type { ModrinthModpack } from '../types/events';

export function useModrinth() {
  const modpackSearchQuery = ref('');
  const selectedGameVersion = ref<string | null>(null);
  const selectedLoader = ref<string | null>(null);
  const selectedCategory = ref<string | null>(null);
  const sortBy = ref('relevance');
  const modpacks = ref<ModrinthModpack[]>([]);
  const loadingModpacks = ref(false);
  const modpackCurrentPage = ref(1);
  const modpackItemsPerPage = 12;
  const modpackTotalHits = ref(0);
  const modpackInitialLoad = ref(true);

  const modpackTotalPages = computed(() => Math.ceil(modpackTotalHits.value / modpackItemsPerPage));

  const gameVersions = ref<string[]>([]);
  const loaders = ['fabric', 'forge', 'quilt', 'neoforge'];
  // value 是 Modrinth 的英文分类 id，title 只用于界面展示。
  // 这里只能用整合包分类（/v2/tag/category 里 project_type=modpack 的那一组），
  // 模组分类（decoration、worldgen 之类）拿去筛整合包基本查不出东西。
  const categoryOptions = [
    { title: '冒险', value: 'adventure' },
    { title: '高难度', value: 'challenging' },
    { title: '战斗', value: 'combat' },
    { title: '大杂烩', value: 'kitchen-sink' },
    { title: '轻量', value: 'lightweight' },
    { title: '魔法', value: 'magic' },
    { title: '多人', value: 'multiplayer' },
    { title: '优化', value: 'optimization' },
    { title: '任务', value: 'quests' },
    { title: '科技', value: 'technology' },
  ];
  const modpackSortOptions = [
    { title: '相关性', value: 'relevance' },
    { title: '下载量', value: 'downloads' },
    { title: '关注度', value: 'follows' },
    { title: '最新', value: 'newest' },
    { title: '更新', value: 'updated' },
  ];

  async function fetchGameVersions() {
    try {
      const manifest = await api.version.getVersions();
      gameVersions.value = manifest.versions
        .filter((v) => v.type === 'release')
        .map((v) => v.id)
        .sort(compareVersionDesc);
    } catch (error) {
      logError('Failed to fetch game versions for filter', error, 'useModrinth');
    }
  }

  async function searchModpacksInternal() {
    loadingModpacks.value = true;
    try {
      const offset = (modpackCurrentPage.value - 1) * modpackItemsPerPage;

      const result = await api.modpack.searchModrinthModpacks({
        query: modpackSearchQuery.value || undefined,
        gameVersions: selectedGameVersion.value ? [selectedGameVersion.value] : undefined,
        loaders: selectedLoader.value ? [selectedLoader.value] : undefined,
        categories: selectedCategory.value ? [selectedCategory.value] : undefined,
        limit: modpackItemsPerPage,
        offset: offset,
        sortBy: sortBy.value,
      });

      modpacks.value = result.hits || [];
      modpackTotalHits.value = result.total_hits || 0;
      modpackInitialLoad.value = false;
    } catch (error) {
      logError('搜索整合包失败', error, 'useModrinth');
      modpacks.value = [];
      modpackTotalHits.value = 0;
      modpackInitialLoad.value = false;
    } finally {
      loadingModpacks.value = false;
    }
  }

  // 防抖搜索（300ms）
  const debouncedSearch = useDebounceFn(searchModpacksInternal, 300);

  // 立即搜索（用于分页、筛选器变化等）
  async function searchModpacks() {
    await searchModpacksInternal();
  }

  // 防抖搜索（用于输入框输入）
  function searchModpacksDebounced() {
    debouncedSearch.call();
  }

  // 监听搜索关键词变化，自动防抖搜索
  watch(modpackSearchQuery, () => {
    if (!modpackInitialLoad.value) {
      modpackCurrentPage.value = 1;
      searchModpacksDebounced();
    }
  });

  // 监听筛选器变化，立即搜索
  watch([selectedGameVersion, selectedLoader, selectedCategory, sortBy], () => {
    if (!modpackInitialLoad.value) {
      modpackCurrentPage.value = 1;
      searchModpacks();
    }
  });

  async function onModpackPageChange(page: number) {
    modpackCurrentPage.value = page;
    await searchModpacks();
  }

  return {
    modpackSearchQuery,
    selectedGameVersion,
    selectedLoader,
    selectedCategory,
    sortBy,
    modpacks,
    loadingModpacks,
    modpackCurrentPage,
    modpackTotalHits,
    modpackTotalPages,
    modpackInitialLoad,
    gameVersions,
    loaders,
    categoryOptions,
    modpackSortOptions,
    fetchGameVersions,
    searchModpacks,
    searchModpacksDebounced,
    onModpackPageChange,
  };
}
