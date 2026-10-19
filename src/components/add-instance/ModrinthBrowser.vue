<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { useModrinth } from '../../composables/useModrinth';
import type { ModrinthModpack } from '../../types/events';
import ModpackCard from './ModpackCard.vue';

const router = useRouter();
const { t } = useI18n();
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
  categoryOptions,
  modpackSortOptions,
  fetchGameVersions,
  searchModpacks,
  onModpackPageChange,
} = useModrinth();

function selectModpack(modpack: ModrinthModpack) {
  selectedModpack.value = modpack;
  router.push({
    path: '/install-modpack',
    query: {
      projectId: modpack.slug,
      title: modpack.title,
    },
  });
}

onMounted(async () => {
  await fetchGameVersions();
  await searchModpacks();
});
</script>

<template>
  <div>
    <v-card variant="flat" color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <v-row density="compact">
          <v-col cols="12" sm="6">
            <v-text-field
              density="comfortable"
              v-model="modpackSearchQuery"
              :label="t('modpackBrowser.searchLabel')"
              hide-details
              clearable
            >
              <template #prepend-inner>
                <v-icon size="20" color="on-surface-variant">mdi-magnify</v-icon>
              </template>
            </v-text-field>
          </v-col>
          <v-col cols="6" sm="3">
            <v-select
              density="comfortable"
              v-model="selectedGameVersion"
              :items="gameVersions"
              :label="t('modpackBrowser.gameVersionLabel')"
              clearable
              hide-details
            >
              <template #prepend-inner>
                <v-icon size="20" color="on-surface-variant">mdi-minecraft</v-icon>
              </template>
            </v-select>
          </v-col>
          <v-col cols="6" sm="3">
            <v-select
              density="comfortable"
              v-model="selectedLoader"
              :items="loaders"
              :label="t('modpackBrowser.loaderLabel')"
              clearable
              hide-details
            >
              <template #prepend-inner>
                <v-icon size="20" color="on-surface-variant">mdi-puzzle</v-icon>
              </template>
            </v-select>
          </v-col>
        </v-row>

        <v-row density="compact" class="mt-2">
          <v-col cols="6">
            <v-select
              density="comfortable"
              v-model="selectedCategory"
              :items="categoryOptions"
              :label="t('modpackBrowser.categoryLabel')"
              clearable
              hide-details
            >
              <template #prepend-inner>
                <v-icon size="20" color="on-surface-variant">mdi-tag</v-icon>
              </template>
            </v-select>
          </v-col>
          <v-col cols="6">
            <v-select
              density="comfortable"
              v-model="sortBy"
              :items="modpackSortOptions"
              :label="t('modpackBrowser.sortLabel')"
              hide-details
            >
              <template #prepend-inner>
                <v-icon size="20" color="on-surface-variant">mdi-sort</v-icon>
              </template>
            </v-select>
          </v-col>
        </v-row>
      </v-card-text>
    </v-card>

    <div v-if="loadingModpacks" class="text-center py-12">
      <v-progress-circular indeterminate size="48" color="primary" />
      <div class="text-body-medium text-on-surface-variant mt-4">{{ t('modpackBrowser.searching') }}</div>
    </div>

    <div v-else-if="modpacks.length === 0" class="text-center py-12">
      <v-avatar size="80" color="surface-container-high" class="mb-4">
        <v-icon size="40" color="on-surface-variant">mdi-package-variant</v-icon>
      </v-avatar>
      <div class="text-body-large font-weight-medium">
        {{ modpackInitialLoad ? t('modpackBrowser.loading') : t('modpackBrowser.empty') }}
      </div>
      <div class="text-body-medium text-on-surface-variant">
        {{ modpackInitialLoad ? '' : t('modpackBrowser.emptyHint') }}
      </div>
    </div>

    <template v-else>
      <v-row density="compact">
        <v-col v-for="modpack in modpacks" :key="modpack.slug" cols="12" sm="6" md="4">
          <ModpackCard :modpack="modpack" :selected="selectedModpack?.slug === modpack.slug" @select="selectModpack" />
        </v-col>
      </v-row>

      <div v-if="modpackTotalPages > 1" class="d-flex flex-column align-center mt-5">
        <v-pagination
          v-model="modpackCurrentPage"
          :length="modpackTotalPages"
          :total-visible="5"
          density="comfortable"
          color="primary"
          @update:model-value="onModpackPageChange"
        />
        <div class="text-body-small text-on-surface-variant mt-2">
          {{ t('modpackBrowser.totalHits', { count: modpackTotalHits }, modpackTotalHits) }}
        </div>
      </div>
    </template>
  </div>
</template>
