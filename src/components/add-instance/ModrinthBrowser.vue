<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useRouter } from 'vue-router';
import { useModrinth, type ModrinthModpack } from '../../composables/useModrinth';
import ModpackCard from './ModpackCard.vue';

const router = useRouter();
const selectedModpack = ref<ModrinthModpack | null>(null);

const {
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
  categories,
  modpackSortOptions,
  fetchGameVersions,
  searchModpacks,
  onModpackPageChange
} = useModrinth();

function selectModpack(modpack: ModrinthModpack) {
  selectedModpack.value = modpack;
  router.push({
    path: '/install-modpack',
    query: {
      projectId: modpack.slug,
      title: modpack.title
    }
  });
}

onMounted(async () => {
  await fetchGameVersions();
});
</script>

<template>
  <div>
    <!-- 搜索和筛选 -->
    <v-card color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <v-row dense>
          <v-col cols="12" sm="6">
            <v-text-field
              v-model="modpackSearchQuery"
              placeholder="搜索整合包..."
              hide-details
              clearable
              @input="searchModpacks"
            >
              <template #prepend-inner>
                <v-icon size="20" color="on-surface-variant">mdi-magnify</v-icon>
              </template>
            </v-text-field>
          </v-col>
          <v-col cols="6" sm="3">
            <v-select
              v-model="selectedGameVersion"
              :items="gameVersions"
              placeholder="游戏版本"
              clearable
              hide-details
              @update:model-value="searchModpacks"
            >
              <template #prepend-inner>
                <v-icon size="20" color="on-surface-variant">mdi-minecraft</v-icon>
              </template>
            </v-select>
          </v-col>
          <v-col cols="6" sm="3">
            <v-select
              v-model="selectedLoader"
              :items="loaders"
              placeholder="加载器"
              clearable
              hide-details
              @update:model-value="searchModpacks"
            >
              <template #prepend-inner>
                <v-icon size="20" color="on-surface-variant">mdi-puzzle</v-icon>
              </template>
            </v-select>
          </v-col>
        </v-row>

        <v-row dense class="mt-2">
          <v-col cols="6">
            <v-select
              v-model="selectedCategory"
              :items="categories"
              placeholder="分类"
              clearable
              hide-details
              @update:model-value="searchModpacks"
            >
              <template #prepend-inner>
                <v-icon size="20" color="on-surface-variant">mdi-tag</v-icon>
              </template>
            </v-select>
          </v-col>
          <v-col cols="6">
            <v-select
              v-model="sortBy"
              :items="modpackSortOptions"
              placeholder="排序"
              hide-details
              @update:model-value="searchModpacks"
            >
              <template #prepend-inner>
                <v-icon size="20" color="on-surface-variant">mdi-sort</v-icon>
              </template>
            </v-select>
          </v-col>
        </v-row>
      </v-card-text>
    </v-card>

    <!-- 加载状态 -->
    <div v-if="loadingModpacks" class="text-center py-12">
      <v-progress-circular indeterminate size="48" color="primary" />
      <div class="text-body-2 text-on-surface-variant mt-4">搜索整合包中...</div>
    </div>

    <!-- 空状态 -->
    <div v-else-if="modpacks.length === 0" class="text-center py-12">
      <v-avatar size="80" color="surface-container-high" class="mb-4">
        <v-icon size="40" color="on-surface-variant">mdi-package-variant</v-icon>
      </v-avatar>
      <div class="text-body-1 font-weight-medium">
        {{ modpackInitialLoad ? '正在加载...' : '没有找到整合包' }}
      </div>
      <div class="text-body-2 text-on-surface-variant">
        {{ modpackInitialLoad ? '' : '尝试调整搜索条件或输入关键词' }}
      </div>
    </div>

    <!-- 整合包列表 -->
    <template v-else>
      <v-row dense>
        <v-col 
          v-for="modpack in modpacks" 
          :key="modpack.slug" 
          cols="12" 
          sm="6" 
          md="4"
        >
          <ModpackCard 
            :modpack="modpack" 
            :selected="selectedModpack?.slug === modpack.slug"
            @select="selectModpack"
          />
        </v-col>
      </v-row>

      <!-- 分页 -->
      <div v-if="modpackTotalPages > 1" class="d-flex flex-column align-center mt-5">
        <v-pagination
          v-model="modpackCurrentPage"
          :length="modpackTotalPages"
          :total-visible="5"
          density="comfortable"
          color="primary"
          @update:model-value="onModpackPageChange"
        />
        <div class="text-caption text-on-surface-variant mt-2">
          共 {{ modpackTotalHits }} 个整合包
        </div>
      </div>
    </template>
  </div>
</template>
