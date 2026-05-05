<script setup lang="ts">
import { computed } from 'vue';
import type { GameInstance } from '../../types/events';
import { formatLastPlayed, getLoaderIcon, getLoaderColor } from '../../utils/format';

const props = defineProps<{
  instance: GameInstance;
}>();

const emit = defineEmits<{
  (e: 'launch', instance: GameInstance): void;
  (e: 'open-folder', instance: GameInstance): void;
  (e: 'delete', instance: GameInstance): void;
  (e: 'rename', instance: GameInstance): void;
}>();

const loaderColor = computed(() => getLoaderColor(props.instance.loaderType));

const loaderLabel = computed(() => {
  const lt = props.instance.loaderType;
  if (!lt || lt === 'None') return '原版';
  if (lt === 'Modded') return '整合包/模组';
  if (lt === 'Unknown') return '未知加载器';
  return lt;
});
</script>

<template>
  <v-card
    color="surface-container"
    variant="flat"
    class="instance-card h-100 d-flex flex-column"
  >
    <!-- 顶部装饰条 -->
    <div :class="['card-accent', `accent-${(instance.loaderType || 'none').toLowerCase()}`]" />

    <v-card-text class="pa-4 flex-grow-1 d-flex flex-column">
      <!-- 头部：标题和菜单 -->
      <div class="d-flex align-start justify-space-between mb-2">
        <div class="flex-grow-1 min-width-0 mr-2">
          <div class="text-h6 font-weight-bold text-truncate name-text" :title="instance.name">
            {{ instance.name }}
          </div>
          <div class="d-flex align-center flex-wrap ga-2 mt-1">
            <v-chip
              size="x-small"
              :color="loaderColor.color"
              variant="tonal"
              label
              class="font-weight-bold"
            >
              {{ loaderLabel }}
            </v-chip>
            <span class="text-caption text-on-surface-variant font-weight-medium">
              {{ instance.gameVersion || instance.version }}
            </span>
          </div>
        </div>
        
        <v-menu>
          <template v-slot:activator="{ props }">
            <v-btn icon variant="text" size="x-small" v-bind="props" class="mt-1">
              <v-icon size="20">mdi-dots-vertical</v-icon>
            </v-btn>
          </template>
          <v-list density="compact" color="surface-container-high" elevation="4">
            <v-list-item @click="emit('open-folder', instance)">
              <template #prepend>
                <v-icon size="18">mdi-folder-open-outline</v-icon>
              </template>
              <v-list-item-title class="text-body-2">打开文件夹</v-list-item-title>
            </v-list-item>
            <v-list-item @click="emit('rename', instance)">
              <template #prepend>
                <v-icon size="18">mdi-pencil-outline</v-icon>
              </template>
              <v-list-item-title class="text-body-2">重命名</v-list-item-title>
            </v-list-item>
            <v-divider class="my-1" />
            <v-list-item @click="emit('delete', instance)" color="error">
              <template #prepend>
                <v-icon size="18" color="error">mdi-delete-outline</v-icon>
              </template>
              <v-list-item-title class="text-body-2">删除</v-list-item-title>
            </v-list-item>
          </v-list>
        </v-menu>
      </div>

      <!-- 中间信息：最后游玩和 Mod 数量 -->
      <div class="info-row d-flex align-center ga-4 text-caption text-on-surface-variant mt-auto pt-4 mb-4">
        <div class="d-flex align-center">
          <v-icon size="14" class="mr-1.5" color="on-surface-variant">mdi-clock-outline</v-icon>
          {{ formatLastPlayed(instance.lastPlayed) }}
        </div>
        <div v-if="instance.modCount" class="d-flex align-center">
          <v-icon size="14" class="mr-1.5" color="on-surface-variant">mdi-puzzle-outline</v-icon>
          {{ instance.modCount }} Mods
        </div>
      </div>

      <!-- 底部：启动按钮 -->
      <v-btn
        variant="tonal"
        :color="loaderColor.color"
        block
        class="launch-btn font-weight-bold"
        @click="emit('launch', instance)"
      >
        <v-icon start size="18" class="mr-2">mdi-play</v-icon>
        开始游戏
      </v-btn>
    </v-card-text>
  </v-card>
</template>

<style scoped>
.instance-card {
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  border: 1px solid rgba(var(--v-theme-on-surface), 0.05);
  border-radius: 16px !important;
  overflow: hidden;
}

.instance-card:hover {
  transform: translateY(-2px);
  background-color: rgb(var(--v-theme-surface-container-high)) !important;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
}

.card-accent {
  height: 4px;
  width: 100%;
}

.name-text {
  line-height: 1.2;
  color: rgb(var(--v-theme-on-surface));
}

.launch-btn {
  border-radius: 12px !important;
  height: 40px !important;
  letter-spacing: 1px;
}

.info-row {
  border-top: 1px dashed rgba(var(--v-theme-on-surface), 0.1);
}

.min-width-0 {
  min-width: 0;
}

/* 加载器特定装饰色 */
.accent-forge { background: linear-gradient(90deg, #3273a8, #5ea3d8); }
.accent-fabric { background: linear-gradient(90deg, #dbb076, #f4d09c); }
.accent-quilt { background: linear-gradient(90deg, #8454b5, #af87db); }
.accent-neoforge { background: linear-gradient(90deg, #e09240, #ffb86c); }
.accent-modded { background: linear-gradient(90deg, #2d9687, #4fc3b3); }
.accent-unknown { background: linear-gradient(90deg, #757575, #9e9e9e); }
.accent-none { background: linear-gradient(90deg, rgb(var(--v-theme-primary)), rgb(var(--v-theme-primary-container))); }
</style>