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
  <v-card variant="outlined" rounded="xl" class="instance-card h-100">
    <v-card-text class="pa-4">
      <!-- 头部：图标和名称 -->
      <div class="d-flex align-center mb-3">
        <v-avatar size="44" class="mr-3 avatar-outlined">
          <v-icon size="22">{{ getLoaderIcon(instance.loader_type) }}</v-icon>
        </v-avatar>
        <div class="flex-grow-1 overflow-hidden">
          <div class="text-subtitle-1 font-weight-bold text-truncate">
            {{ instance.name }}
          </div>
          <div class="text-body-2 text-medium-emphasis">
            {{ instance.loader_type && instance.loader_type !== 'None' ? instance.loader_type + ' ' : '' }}{{ instance.game_version || instance.version }}
          </div>
        </div>
        <v-menu>
          <template v-slot:activator="{ props }">
            <v-btn icon variant="text" size="small" v-bind="props">
              <v-icon size="20">mdi-dots-vertical</v-icon>
            </v-btn>
          </template>
          <v-list density="compact" rounded="lg">
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
              <v-list-item-title class="text-body-2 text-error">删除实例</v-list-item-title>
            </v-list-item>
          </v-list>
        </v-menu>
      </div>

      <!-- 最后运行时间 -->
      <div class="d-flex align-center text-caption text-medium-emphasis mb-3">
        <v-icon size="14" class="mr-1">mdi-clock-outline</v-icon>
        {{ formatLastPlayed(instance.last_played) }}
      </div>

      <!-- 启动按钮 -->
      <v-btn
        variant="outlined"
        rounded="lg"
        block
        @click="emit('launch', instance)"
      >
        <v-icon start size="18">mdi-play</v-icon>
        启动
      </v-btn>
    </v-card-text>
  </v-card>
</template>

<style scoped>
.instance-card {
  transition: transform 0.2s ease, border-color 0.2s ease;
}

.instance-card:hover {
  transform: translateY(-3px);
}

.avatar-outlined {
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
}
</style>
