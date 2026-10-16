<script setup lang="ts">
import { computed } from 'vue';

/**
 * 用整张皮肤裁出脸部方块。皮肤是 64×64（旧版 64×32），头部正面在 (8,8) 的 8×8，
 * 帽层正面在 (40,8)，两层同尺寸叠在一起才是游戏里看到的那张脸。
 * background-size 只锁宽度、高度用 auto 保比例，所以两种尺寸的皮肤都能对齐到同一个头部区域。
 */
const props = withDefaults(
  defineProps<{
    skinUrl: string;
    size?: number;
  }>(),
  { size: 32 },
);

// URL 来自 Mojang 接口且要拼进内联样式，先过一道官方贴图域名的白名单
const safeUrl = computed(() => {
  try {
    const url = new URL(props.skinUrl);
    return url.protocol === 'https:' && url.hostname === 'textures.minecraft.net' ? url.href : '';
  } catch {
    return '';
  }
});

const FACE_PX = 8;
const style = computed(() => {
  const scale = props.size / FACE_PX;
  // 负偏移把目标方块推到容器左上角；第一层是帽层 (40,8)，第二层是本体 (8,8)
  const offset = (x: number) => `${-x * scale}px ${-8 * scale}px`;
  return {
    width: `${props.size}px`,
    height: `${props.size}px`,
    backgroundImage: `url("${safeUrl.value}"), url("${safeUrl.value}")`,
    backgroundSize: `${64 * scale}px auto`,
    backgroundPosition: `${offset(40)}, ${offset(8)}`,
  };
});
</script>

<template>
  <div v-if="safeUrl" class="player-head" :style="style" />
</template>

<style scoped>
.player-head {
  flex-shrink: 0;
  background-repeat: no-repeat;
  image-rendering: pixelated;
}
</style>
