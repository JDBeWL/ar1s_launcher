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
    variant="outlined"
    rounded="lg"
    class="modpack-card h-100"
    :class="{ 'modpack-card--selected': selected }"
    @click="emit('select', modpack)"
  >
    <v-card-text class="pa-3">
      <div class="d-flex align-start mb-2">
        <!-- 图标 -->
        <v-avatar
          size="48"
          rounded="lg"
          class="mr-3 flex-shrink-0"
          :class="modpack.icon_url ? '' : 'avatar-outlined'"
        >
          <v-img v-if="modpack.icon_url" :src="modpack.icon_url" cover />
          <v-icon v-else size="24">mdi-package-variant</v-icon>
        </v-avatar>

        <!-- 标题和作者 -->
        <div class="flex-grow-1 min-width-0">
          <div class="text-subtitle-2 font-weight-bold text-truncate">{{ modpack.title }}</div>
          <div class="text-caption text-medium-emphasis">{{ modpack.author }}</div>
        </div>
      </div>

      <!-- 描述 -->
      <div class="text-caption text-medium-emphasis mb-2 modpack-desc">
        {{ truncateText(modpack.description, 80) }}
      </div>

      <!-- 标签 -->
      <div class="d-flex flex-wrap ga-1 mb-2">
        <v-chip
          v-for="loader in modpack.loaders.slice(0, 2)"
          :key="loader"
          size="x-small"
          variant="outlined"
        >
          {{ loader }}
        </v-chip>
        <v-chip
          v-if="modpack.game_versions.length > 0"
          size="x-small"
          variant="outlined"
        >
          {{ modpack.game_versions[0] }}
        </v-chip>
      </div>

      <!-- 统计 -->
      <div class="d-flex align-center text-caption text-medium-emphasis">
        <v-icon size="14" class="mr-1">mdi-download</v-icon>
        <span>{{ formatNumber(modpack.downloads) }}</span>
      </div>
    </v-card-text>
  </v-card>
</template>

<style scoped>
.modpack-card {
  cursor: pointer;
  transition: transform 0.2s ease, border-color 0.2s ease;
}

.modpack-card:hover {
  transform: translateY(-2px);
}

.modpack-card--selected {
  border-color: rgb(var(--v-theme-on-surface));
}

.avatar-outlined {
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
}

.min-width-0 {
  min-width: 0;
}

.modpack-desc {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  line-height: 1.4;
}
</style>
