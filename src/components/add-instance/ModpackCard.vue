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
  if (num >= 1000000) {
    return (num / 1000000).toFixed(1) + 'M';
  }
  if (num >= 1000) {
    return (num / 1000).toFixed(1) + 'K';
  }
  return num.toString();
}

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString();
}

function truncateText(text: string, length: number): string {
  if (!text) return '';
  if (text.length <= length) return text;
  return text.substring(0, length) + '...';
}

function formatVersionRange(versions: string[]): string {
  if (!versions || versions.length === 0) return '未知';
  if (versions.length === 1) return versions[0];
  // 简单的显示第一个和最后一个，或者只显示数量
  return `${versions[0]} - ${versions[versions.length - 1]}`;
}

function formatLoaders(loaders: string[]): string {
  if (!loaders || loaders.length === 0) return '未知';
  return loaders.join(', ');
}
</script>

<template>
  <v-card 
    class="modpack-card" 
    elevation="2"
    @click="emit('select', modpack)"
    :class="{ 'modpack-card-selected': selected }"
  >
    <v-img
      v-if="modpack.icon_url"
      :src="modpack.icon_url"
      height="120"
      cover
      class="modpack-image"
    ></v-img>
    <div v-else class="modpack-image-placeholder">
      <v-icon size="48" color="grey">mdi-package-variant</v-icon>
    </div>
    
    <v-card-title class="text-h6 modpack-title">
      {{ modpack.title }}
    </v-card-title>
    
    <v-card-text class="modpack-info">
      <div class="modpack-author">作者: {{ modpack.author }}</div>
      <div class="modpack-downloads">下载量: {{ formatNumber(modpack.downloads) }}</div>
      <div class="modpack-versions">
        支持版本: {{ formatVersionRange(modpack.game_versions) }}
      </div>
      <div class="modpack-loaders">
        加载器: {{ formatLoaders(modpack.loaders) }}
      </div>
      <div class="modpack-categories" v-if="modpack.categories.length > 0">
        分类: {{ modpack.categories.slice(0, 3).join(', ') }}
      </div>
      <div class="modpack-updated">
        更新: {{ formatDate(modpack.date_modified) }}
      </div>
      <div class="modpack-description">
        {{ truncateText(modpack.description, 100) }}
      </div>
    </v-card-text>
  </v-card>
</template>

<style scoped>
.modpack-card {
  height: 100%;
  display: flex;
  flex-direction: column;
  cursor: pointer;
  transition: all 0.2s ease;
  border: 2px solid transparent;
}

.modpack-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 8px 16px rgba(0,0,0,0.1);
}

.modpack-card-selected {
  border-color: rgb(var(--v-theme-primary));
  background-color: rgba(var(--v-theme-primary), 0.05);
}

.modpack-image-placeholder {
  height: 120px;
  background-color: #f5f5f5;
  display: flex;
  align-items: center;
  justify-content: center;
}

:deep(.v-theme--dark) .modpack-image-placeholder {
  background-color: #333;
}

.modpack-title {
  font-size: 1.1rem !important;
  line-height: 1.4;
  padding-bottom: 4px;
}

.modpack-info {
  flex-grow: 1;
  font-size: 0.85rem;
  line-height: 1.5;
  color: rgba(var(--v-theme-on-surface), 0.7);
}

.modpack-description {
  margin-top: 8px;
  font-size: 0.8rem;
  color: rgba(var(--v-theme-on-surface), 0.6);
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
</style>
