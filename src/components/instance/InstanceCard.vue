<script setup lang="ts">
import type { GameInstance } from '../../types/events';

defineProps<{
  instance: GameInstance;
}>();

const emit = defineEmits<{
  (e: 'launch', instance: GameInstance): void;
  (e: 'open-folder', instance: GameInstance): void;
  (e: 'delete', instance: GameInstance): void;
  (e: 'rename', instance: GameInstance): void;
}>();

function formatLastPlayed(time?: number) {
  if (!time) return '从未运行';
  return new Date(time).toLocaleString();
}

function getLoaderIcon(loaderType?: string) {
  if (!loaderType || loaderType === 'None') return 'mdi-minecraft';
  switch (loaderType.toLowerCase()) {
    case 'forge': return 'mdi-anvil';
    case 'fabric': return 'mdi-texture-box';
    case 'quilt': return 'mdi-quilt';
    case 'neoforge': return 'mdi-anvil';
    default: return 'mdi-minecraft';
  }
}
</script>

<template>
  <v-card color="surface-container" class="instance-card h-100">
    <v-card-text class="pa-4">
      <!-- 头部：图标和名称 -->
      <div class="d-flex align-center mb-3">
        <v-avatar size="48" color="primary-container" class="mr-3">
          <v-icon size="24" color="on-primary-container">{{ getLoaderIcon(instance.loaderType) }}</v-icon>
        </v-avatar>
        <div class="flex-grow-1 overflow-hidden">
          <div class="text-subtitle-1 font-weight-bold text-truncate">
            {{ instance.name }}
          </div>
          <div class="text-body-2 text-on-surface-variant">
            {{ instance.loaderType && instance.loaderType !== 'None' ? instance.loaderType + ' ' : '' }}{{ instance.gameVersion || instance.version }}
          </div>
        </div>
        <v-menu>
          <template v-slot:activator="{ props }">
            <v-btn icon variant="text" size="small" v-bind="props">
              <v-icon size="20">mdi-dots-vertical</v-icon>
            </v-btn>
          </template>
          <v-list density="compact" color="surface-container-high">
            <v-list-item @click="emit('open-folder', instance)">
              <template #prepend>
                <v-icon size="20" color="on-surface-variant">mdi-folder-open</v-icon>
              </template>
              <v-list-item-title class="text-body-2">打开文件夹</v-list-item-title>
            </v-list-item>
            <v-list-item @click="emit('rename', instance)">
              <template #prepend>
                <v-icon size="20" color="on-surface-variant">mdi-pencil</v-icon>
              </template>
              <v-list-item-title class="text-body-2">重命名</v-list-item-title>
            </v-list-item>
            <v-divider class="my-1" />
            <v-list-item @click="emit('delete', instance)">
              <template #prepend>
                <v-icon size="20" color="error">mdi-delete</v-icon>
              </template>
              <v-list-item-title class="text-body-2 text-error">删除实例</v-list-item-title>
            </v-list-item>
          </v-list>
        </v-menu>
      </div>

      <!-- 最后运行时间 -->
      <div class="d-flex align-center text-caption text-on-surface-variant mb-4">
        <v-icon size="14" class="mr-1">mdi-clock-outline</v-icon>
        {{ formatLastPlayed(instance.lastPlayed) }}
      </div>

      <!-- 启动按钮 -->
      <v-btn
        variant="tonal"
        color="primary"
        block
        @click="emit('launch', instance)"
      >
        <v-icon start size="20">mdi-play</v-icon>
        启动
      </v-btn>
    </v-card-text>
  </v-card>
</template>

<style scoped>
.instance-card {
  transition: transform 0.2s cubic-bezier(0.4, 0, 0.2, 1),
              box-shadow 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

.instance-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
}
</style>
