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
  // Initial search if needed, or wait for user input
  // searchModpacks(); 
});
</script>

<template>
  <div>
    <!-- 搜索框和游戏版本 -->
    <v-row no-gutters class="align-center mb-4">
      <v-col class="flex-grow-1 pr-2">
        <v-text-field
          v-model="modpackSearchQuery"
          label="搜索整合包"
          prepend-inner-icon="mdi-magnify"
          clearable
          hide-details
          @input="searchModpacks"
        ></v-text-field>
      </v-col>
      <v-col class="shrink pr-2" style="max-width: 200px">
        <v-select
          v-model="selectedGameVersion"
          :items="gameVersions"
          label="游戏版本"
          clearable
          hide-details
          @update:model-value="searchModpacks"
        ></v-select>
      </v-col>
      <v-col class="shrink" style="max-width: 200px">
        <v-select
          v-model="selectedLoader"
          :items="loaders"
          label="加载器"
          clearable
          hide-details
          @update:model-value="searchModpacks"
        ></v-select>
      </v-col>
    </v-row>

    <!-- 其他筛选条件 -->
    <v-row no-gutters class="align-center mb-4">
      <v-col class="flex-grow-1 pr-2" style="min-width: 150px">
        <v-select
          v-model="selectedCategory"
          :items="categories"
          label="分类"
          clearable
          hide-details
          @update:model-value="searchModpacks"
        ></v-select>
      </v-col>
      <v-col class="flex-grow-1 pr-2" style="min-width: 150px">
        <v-select
          v-model="sortBy"
          :items="modpackSortOptions"
          label="排序"
          hide-details
          @update:model-value="searchModpacks"
        ></v-select>
      </v-col>
    </v-row>

    <!-- 加载状态 -->
    <v-row v-if="loadingModpacks" class="mb-4">
      <v-col cols="12" class="text-center">
        <v-progress-circular indeterminate color="primary"></v-progress-circular>
        <div class="mt-2">正在搜索整合包...</div>
      </v-col>
    </v-row>

    <!-- 整合包列表 -->
    <v-row v-else-if="modpacks.length > 0" class="mb-4">
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

    <!-- 分页组件 -->
    <v-row v-if="modpacks.length > 0 && modpackTotalPages > 1" class="mb-4">
      <v-col cols="12" class="d-flex justify-center">
        <div class="d-flex align-center">
          <v-pagination
            v-model="modpackCurrentPage"
            :length="modpackTotalPages"
            :total-visible="7"
            @update:model-value="onModpackPageChange"
            class="mr-4"
          ></v-pagination>
          <div class="text-caption text-grey">
            共 {{ modpackTotalHits }} 个整合包
          </div>
        </div>
      </v-col>
    </v-row>

    <!-- 空状态 -->
    <v-row v-else-if="!loadingModpacks && modpacks.length === 0" class="mb-4">
      <v-col cols="12" class="text-center">
        <v-icon size="64" color="grey">mdi-package-variant</v-icon>
        <div class="mt-2 text-grey">
          {{ modpackInitialLoad ? '正在加载热门整合包...' : (modpackSearchQuery || selectedGameVersion || selectedLoader || selectedCategory ? '未找到相关整合包' : '请输入关键词搜索整合包') }}
        </div>
      </v-col>
    </v-row>
  </div>
</template>
