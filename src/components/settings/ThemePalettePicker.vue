<script setup lang="ts">
/**
 * 主题色选择器
 *
 * 预设渲染成各自的主色圆点；色块固定取浅色 tone 40，
 * 深色模式的浅色调主色偏粉白，十颗摆一起分不出色相。
 */
import { computed } from 'vue';
import { buildTheme } from '../../theme/md3';
import { THEME_PRESETS } from '../../theme/presets';

const props = defineProps<{ modelValue: string }>();
const emit = defineEmits<{ 'update:modelValue': [value: string] }>();

const swatches = computed(() =>
  THEME_PRESETS.map((preset) => {
    const colors = buildTheme(preset.seed).light;
    return {
      id: preset.id,
      label: preset.label,
      primary: colors.primary,
      onPrimary: colors['on-primary'],
    };
  }),
);
</script>

<template>
  <div class="palette-grid" role="radiogroup" aria-label="主题色方案">
    <button
      v-for="swatch in swatches"
      :key="swatch.id"
      type="button"
      role="radio"
      class="palette-item"
      :class="{ active: swatch.id === props.modelValue }"
      :aria-checked="swatch.id === props.modelValue"
      :style="{ '--swatch': swatch.primary }"
      @click="emit('update:modelValue', swatch.id)"
    >
      <span class="swatch" :style="{ background: swatch.primary, color: swatch.onPrimary }">
        <v-icon v-if="swatch.id === props.modelValue" size="18">mdi-check</v-icon>
      </span>
      <span class="label">{{ swatch.label }}</span>
    </button>
  </div>
</template>

<style scoped>
.palette-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(62px, 1fr));
  gap: 14px 6px;
}

.palette-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 2px 0;
  border: 0;
  background: none;
  color: inherit;
  cursor: pointer;
}

.swatch {
  display: grid;
  place-items: center;
  width: 40px;
  height: 40px;
  border-radius: 50%;
  box-shadow: inset 0 0 0 1px rgba(var(--v-theme-on-surface), 0.14);
  transition:
    transform 0.15s ease,
    box-shadow 0.15s ease;
}

.palette-item:hover .swatch {
  transform: scale(1.06);
}

/* 选中环用该配色自己的主色，预览的就是即将生效的颜色 */
.palette-item.active .swatch {
  box-shadow:
    0 0 0 2px rgb(var(--v-theme-surface-container)),
    0 0 0 4px var(--swatch);
}

.palette-item:focus-visible {
  outline: none;
}

.palette-item:focus-visible .swatch {
  box-shadow:
    0 0 0 2px rgb(var(--v-theme-surface-container)),
    0 0 0 4px rgb(var(--v-theme-on-surface));
}

.label {
  font-size: 12px;
  line-height: 1.2;
  color: rgba(var(--v-theme-on-surface), 0.7);
  white-space: nowrap;
}

.palette-item.active .label {
  color: rgb(var(--v-theme-on-surface));
  font-weight: 500;
}
</style>
