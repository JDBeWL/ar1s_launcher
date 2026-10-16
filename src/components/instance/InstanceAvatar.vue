<script setup lang="ts">
import { computed } from 'vue';
import type { GameInstance } from '../../types/events';
import { getLoaderColor, getLoaderIcon } from '../../utils/format';

/** 原版实例统一用项目自带的草方块图标 */
const VANILLA_ICON = '/icons/grass_block.svg';

const props = withDefaults(
  defineProps<{
    instance: GameInstance;
    size?: number;
  }>(),
  {
    size: 44,
  },
);

const isVanilla = computed(() => !props.instance.loaderType || props.instance.loaderType === 'None');
const iconUrl = computed(() => props.instance.iconUrl || (isVanilla.value ? VANILLA_ICON : ''));
const loaderColor = computed(() => getLoaderColor(props.instance.loaderType));
const loaderIcon = computed(() => getLoaderIcon(props.instance.loaderType));
const glyphSize = computed(() => Math.round(props.size * 0.5));
</script>

<template>
  <v-avatar :size="size" :color="iconUrl ? 'transparent' : loaderColor.bgColor">
    <v-img v-if="iconUrl" :src="iconUrl" cover>
      <template #error>
        <v-icon :size="glyphSize" :color="loaderColor.color">{{ loaderIcon }}</v-icon>
      </template>
    </v-img>
    <v-icon v-else :size="glyphSize" :color="loaderColor.color">{{ loaderIcon }}</v-icon>
  </v-avatar>
</template>
