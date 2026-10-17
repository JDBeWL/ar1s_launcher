/**
 * 崩溃诊断弹窗状态
 * 每次崩溃整份替换上一次的诊断，原始输出默认折叠
 */
import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { CrashDiagnosis } from '../types/events';

export const useCrashStore = defineStore('crash', () => {
  const visible = ref(false);
  const diagnosis = ref<CrashDiagnosis | null>(null);
  const showRaw = ref(false);

  function open(payload: CrashDiagnosis) {
    diagnosis.value = payload;
    showRaw.value = false;
    visible.value = true;
  }

  function close() {
    visible.value = false;
  }

  function toggleRaw() {
    showRaw.value = !showRaw.value;
  }

  return { visible, diagnosis, showRaw, open, close, toggleRaw };
});
