<script setup lang="ts">
import type { ModrinthModpack } from '../../composables/useModrinth';

defineProps<{
  modpack: ModrinthModpack
  selected: boolean
}>()

const emit = defineEmits<{
  (e: 'select', modpack: ModrinthModpack): void
}>()

function formatNumber(num: number): string {
  if (num >= 1000000) return (num / 1000000).toFixed(1) + 'M';
  if (num >= 1000) return (num / 1000).toFixed(1) + 'K';
  return num.toString();
}

function truncateText(text: string, length: number): string {
  if (!text) return '';
  if (text.length <= length) return text;
  return text.substring(0, length) + '...';
}
</script>

<template>
  <v-card 
    color="surface-container"
    class="modpack-card h-100"
    :class="{ 'modpack-card--selected': selected }"
    @click="emit('select', modpack)"
  >
    <v-card-text class="pa-4">
      <div class="d-flex align-start mb-3">
        <!-- 图标 -->
        <v-avatar
          size="56"
          rounded="lg"
          class="mr-3 flex-shrink-0"
          :color="modpack.icon_url ? undefined : 'secondary-container'"
        >
          <v-img v-if="modpack.icon_url" :src="modpack.icon_url" cover />
          <v-icon v-else size="28" color="on-secondary-container">mdi-package-variant</v-icon>
        </v-avatar>

        <!-- 标题和作者 -->
        <div class="flex-grow-1 min-width-0">
          <div class="text-subtitle-1 font-weight-bold text-truncate">{{ modpack.title }}</div>
          <div class="text-caption text-on-surface-variant">{{ modpack.author }}</div>
        </div>
      </div>

      <!-- 描述 -->
      <div class="text-body-2 text-on-surface-variant mb-3 modpack-desc">
        {{ truncateText(modpack.description, 80) }}
      </div>

      <!-- 标签 -->
      <div class="d-flex flex-wrap ga-1 mb-3">
        <v-chip
          v-for="loader in modpack.loaders.slice(0, 2)"
          :key="loader"
          size="small"
          color="primary"
          variant="tonal"
        >
          {{ loader }}
        </v-chip>
        <v-chip
          v-if="modpack.game_versions.length > 0"
          size="small"
          color="secondary"
          variant="tonal"
        >
          {{ modpack.game_versions[0] }}
        </v-chip>
      </div>

      <!-- 统计 -->
      <div class="d-flex align-center text-caption text-on-surface-variant">
        <v-icon size="16" class="mr-1">mdi-download</v-icon>
        <span>{{ formatNumber(modpack.downloads) }}</span>
      </div>
    </v-card-text>
  </v-card>
</template>

<style scoped>
.modpack-card {
  cursor: pointer;
  transition: transform 0.2s cubic-bezier(0.4, 0, 0.2, 1),
              box-shadow 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

.modpack-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
}

.modpack-card--selected {
  border: 2px solid rgb(var(--v-theme-primary));
}

.min-width-0 {
  min-width: 0;
}

.modpack-desc {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  line-height: 1.5;
}
</style>
