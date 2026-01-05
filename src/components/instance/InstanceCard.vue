<script setup lang="ts">
import type { GameInstance } from '../../types/events';
import { formatLastPlayed, getLoaderIcon } from '../../utils/format';

defineProps<{
  instance: GameInstance;
}>();

const emit = defineEmits<{
  (e: 'launch', instance: GameInstance): void;
  (e: 'open-folder', instance: GameInstance): void;
  (e: 'delete', instance: GameInstance): void;
  (e: 'rename', instance: GameInstance): void;
}>();
</script>

<template>
  <v-card color="surface-container" variant="flat" class="instance-card h-100">
    <v-card-text class="pa-4">
      <!-- 头部 -->
      <div class="d-flex align-center mb-3">
        <v-avatar size="44" color="primary-container" class="mr-3">
          <v-icon size="22" color="on-primary-container">{{ getLoaderIcon(instance.loaderType) }}</v-icon>
        </v-avatar>
        <div class="flex-grow-1 overflow-hidden">
          <div class="text-body-1 font-weight-bold text-truncate">
            {{ instance.name }}
          </div>
          <div class="text-caption text-on-surface-variant">
            <span v-if="instance.loaderType && instance.loaderType !== 'None'">{{ instance.loaderType }} · </span>
            {{ instance.gameVersion || instance.version }}
          </div>
        </div>
        <v-menu>
          <template v-slot:activator="{ props }">
            <v-btn icon variant="text" size="small" v-bind="props">
              <v-icon size="18">mdi-dots-vertical</v-icon>
            </v-btn>
          </template>
          <v-list density="compact" color="surface-container-high">
            <v-list-item @click="emit('open-folder', instance)">
              <template #prepend>
                <v-icon size="18">mdi-folder-open</v-icon>
              </template>
              <v-list-item-title class="text-body-2">打开文件夹</v-list-item-title>
            </v-list-item>
            <v-list-item @click="emit('rename', instance)">
              <template #prepend>
                <v-icon size="18">mdi-pencil</v-icon>
              </template>
              <v-list-item-title class="text-body-2">重命名</v-list-item-title>
            </v-list-item>
            <v-divider class="my-1" />
            <v-list-item @click="emit('delete', instance)">
              <template #prepend>
                <v-icon size="18" color="error">mdi-delete</v-icon>
              </template>
              <v-list-item-title class="text-body-2 text-error">删除</v-list-item-title>
            </v-list-item>
          </v-list>
        </v-menu>
      </div>

      <!-- 上次启动时间 -->
      <div class="d-flex align-center text-caption text-on-surface-variant mb-3">
        <v-icon size="14" class="mr-1">mdi-clock-outline</v-icon>
        {{ formatLastPlayed(instance.lastPlayed) }}
      </div>

      <!-- 启动按钮 -->
      <v-btn
        variant="tonal"
        color="primary"
        block
        size="small"
        @click="emit('launch', instance)"
      >
        <v-icon start size="16">mdi-play</v-icon>
        启动
      </v-btn>
    </v-card-text>
  </v-card>
</template>

<style scoped>
.instance-card {
  transition: transform 0.15s ease, box-shadow 0.15s ease;
}

.instance-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
}
</style>
