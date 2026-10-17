<script setup lang="ts">
import { computed, ref } from 'vue';
import { useCrashStore } from '../stores/crashStore';
import { useNotificationStore } from '../stores/notificationStore';
import { copyText } from '../utils/clipboard';
import type { CrashFinding } from '../types/events';

const crash = useCrashStore();
const notifications = useNotificationStore();

const showExcerpt = ref(false);

const diagnosis = computed(() => crash.diagnosis);
const findings = computed(() => diagnosis.value?.findings ?? []);
const context = computed(() => diagnosis.value?.context ?? []);
const excerpt = computed(() => diagnosis.value?.excerpt ?? []);

const title = computed(() =>
  diagnosis.value?.recognized ? '启动失败 · 已定位到可疑环节' : '启动失败 · 未识别出已知特征',
);

const rawLineCount = computed(() => diagnosis.value?.rawOutput.split('\n').length ?? 0);

function roleLabel(role: CrashFinding['role']) {
  return role === 'root' ? '根因' : '表象';
}

/** 表象压成中性灰，避免和真正的元凶抢注意力 */
function alertColor(finding: CrashFinding) {
  if (finding.role === 'symptom') return 'grey';
  return finding.confidence === 'high' ? 'error' : 'warning';
}

async function copyReport() {
  if (!diagnosis.value) return;

  if (await copyText(diagnosis.value.reportText)) {
    notifications.success('已复制诊断报告', '结论、证据行和原始输出都在里面，可直接贴给上游');
  } else {
    notifications.error('复制失败，请在弹窗里手动选中文字');
  }
}
</script>

<template>
  <v-dialog :model-value="crash.visible" max-width="760" persistent>
    <v-card v-if="diagnosis" prepend-icon="mdi-alert-circle-outline">
      <template #title>
        <div class="d-flex align-center ga-2">
          <span class="text-error">{{ title }}</span>
          <v-chip size="x-small" variant="tonal" class="flex-shrink-0">退出码 {{ diagnosis.exitCode }}</v-chip>
        </div>
      </template>

      <v-card-text class="crash-body">
        <v-alert
          v-for="(finding, index) in findings"
          :key="index"
          :color="alertColor(finding)"
          variant="tonal"
          density="compact"
          class="mb-3"
        >
          <div class="d-flex align-center ga-2 mb-1">
            <v-chip size="x-small" class="flex-shrink-0 align-self-start mt-1" :color="alertColor(finding)">
              {{ roleLabel(finding.role) }}
            </v-chip>
            <span class="text-body-2 font-weight-medium">{{ finding.conclusion }}</span>
          </div>

          <div v-if="finding.culprits.length" class="mt-1">
            <span class="text-caption">{{ finding.culpritLabel }}：</span>
            <code v-for="culprit in finding.culprits" :key="culprit" class="chip-code">{{ culprit }}</code>
          </div>

          <div v-for="line in finding.evidence" :key="line.lineNo" class="log-line mt-1">
            <span class="log-no">{{ line.lineNo }}</span>
            <span class="log-text">{{ line.text }}</span>
          </div>

          <div class="mt-2 d-flex align-start ga-1">
            <v-icon size="small" class="mt-1">mdi-lightbulb-outline</v-icon>
            <span class="text-body-2">{{ finding.suggestion }}</span>
          </div>
        </v-alert>

        <v-divider class="my-4" />

        <div class="text-subtitle-2 mb-2">环境</div>
        <div v-for="line in context" :key="line.label" class="context-row">
          <span class="context-label">{{ line.label }}</span>
          <span class="context-value">{{ line.value }}</span>
        </div>

        <template v-if="excerpt.length">
          <v-btn
            class="mt-4 text-none"
            size="small"
            variant="text"
            :prepend-icon="showExcerpt ? 'mdi-chevron-up' : 'mdi-chevron-down'"
            @click="showExcerpt = !showExcerpt"
          >
            命中位置上下文（第 {{ excerpt[0].lineNo }}-{{ excerpt[excerpt.length - 1].lineNo }} 行）
          </v-btn>
          <div v-if="showExcerpt" class="raw-block">
            <div v-for="line in excerpt" :key="line.lineNo" class="log-line">
              <span class="log-no">{{ line.lineNo }}</span>
              <span class="log-text">{{ line.text }}</span>
            </div>
          </div>
        </template>

        <v-btn
          class="mt-3 text-none"
          size="small"
          variant="text"
          :prepend-icon="crash.showRaw ? 'mdi-chevron-up' : 'mdi-chevron-down'"
          @click="crash.toggleRaw()"
        >
          原始输出（{{ rawLineCount }} 行）
        </v-btn>
        <pre v-if="crash.showRaw" class="raw-block raw-pre">{{ diagnosis.rawOutput }}</pre>
      </v-card-text>

      <v-divider />

      <v-card-actions class="pa-4">
        <v-btn variant="text" prepend-icon="mdi-content-copy" @click="copyReport">复制诊断报告</v-btn>
        <v-spacer />
        <v-btn color="primary" @click="crash.close()">关闭</v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<style scoped>
.crash-body {
  max-height: 62vh;
  overflow-y: auto;
}

.chip-code {
  display: inline-block;
  margin-right: 4px;
  padding: 0 6px;
  border-radius: 4px;
  background: rgba(0, 0, 0, 0.08);
  font-size: 0.8125rem;
}

.context-row {
  display: flex;
  gap: 8px;
  font-size: 0.8125rem;
  line-height: 1.6;
}

.context-label {
  flex: 0 0 96px;
  opacity: 0.7;
}

.context-value {
  word-break: break-all;
}

.raw-block {
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.05);
  padding: 8px;
  max-height: 30vh;
  overflow-y: auto;
}

.raw-pre {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
  font-size: 0.75rem;
}

.log-line {
  display: flex;
  gap: 8px;
  font-family: monospace;
  font-size: 0.75rem;
}

.log-no {
  flex: 0 0 42px;
  text-align: right;
  opacity: 0.55;
}

.log-text {
  word-break: break-all;
}
</style>
