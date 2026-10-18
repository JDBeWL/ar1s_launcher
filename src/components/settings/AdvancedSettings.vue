<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue';
import { configApi } from '../../services';
import { useSettingsStore } from '../../stores/settings';
import { logError } from '../../utils/logger';
import type { JvmAdvancedOptions, GcStrategy } from '../../types/events';

const settingsStore = useSettingsStore();

const options = ref<JvmAdvancedOptions>({
  gcStrategy: 'g1',
  alwaysPreTouch: false,
  disableExplicitGc: false,
  stringDeduplication: false,
  perfDisableSharedMem: false,
  metaspaceSizeMb: null,
  metaspaceMaxSizeMb: null,
  heapDumpOnOom: false,
  extraArgs: '',
});

const metaspaceEnabled = ref(false);
const isInitialized = ref(false);
const saving = ref(false);
const showPreview = ref(false);
const effectiveArgs = ref<string[]>([]);

const metaspacePresets = [128, 256, 512];
const metaspaceMaxPresets = [256, 512, 768, 1024];

// 上限低于 256MB 时大型整合包很容易直接 Metaspace OOM
const metaspaceTooSmall = computed(
  () => options.value.metaspaceMaxSizeMb !== null && options.value.metaspaceMaxSizeMb < 256,
);

const gcOptions: { value: GcStrategy; title: string; subtitle: string }[] = [
  { value: 'g1', title: 'G1GC（推荐）', subtitle: 'G1GC 调优，兼容性最好' },
  { value: 'default', title: '默认', subtitle: '不指定回收器，完全交给 JVM，与官方启动器一致' },
  { value: 'parallel', title: 'Parallel GC', subtitle: '吞吐量优先，适合核心数较少的机器' },
  { value: 'serial', title: 'Serial GC', subtitle: '单线程回收，一般只用于排查 GC 问题' },
  { value: 'z', title: 'ZGC', subtitle: '极低停顿，适合超大堆；Java 15 以下会自动回退到 G1' },
];

const gcArgsPreview = computed(() => {
  switch (options.value.gcStrategy) {
    case 'g1':
      return '-XX:+UnlockExperimentalVMOptions -XX:+UseG1GC -XX:G1NewSizePercent=20 -XX:G1ReservePercent=20 -XX:MaxGCPauseMillis=50';
    case 'parallel':
      return '-XX:+UseParallelGC';
    case 'serial':
      return '-XX:+UseSerialGC';
    case 'z':
      return '-XX:+UseZGC（Java 21~23 追加 -XX:+ZGenerational；Java 15 以下回退 G1）';
    default:
      return '不添加任何 GC 参数，由 JVM 自行决定';
  }
});

async function loadOptions() {
  try {
    options.value = await configApi.getJvmAdvancedOptions();
    metaspaceEnabled.value = metaspaceConfigured(options.value);
    isInitialized.value = true;
    if (showPreview.value) await refreshPreview();
  } catch (err) {
    logError('Failed to load JVM advanced options', err, 'AdvancedSettings');
  }
}

async function saveOptions() {
  saving.value = true;
  try {
    await configApi.setJvmAdvancedOptions(options.value);
    await refreshPreview();
  } catch (err) {
    logError('Failed to save JVM advanced options', err, 'AdvancedSettings');
  } finally {
    saving.value = false;
  }
}

async function refreshPreview() {
  try {
    effectiveArgs.value = await configApi.getEffectiveJvmArgs(settingsStore.maxMemory);
  } catch (err) {
    logError('Failed to preview JVM args', err, 'AdvancedSettings');
    effectiveArgs.value = [];
  }
}

function metaspaceConfigured(opts: JvmAdvancedOptions): boolean {
  return opts.metaspaceSizeMb !== null || opts.metaspaceMaxSizeMb !== null;
}

function toggleMetaspace(value: boolean | null) {
  const enabled = value === true;
  metaspaceEnabled.value = enabled;
  if (enabled) {
    options.value.metaspaceSizeMb = options.value.metaspaceSizeMb ?? 256;
    options.value.metaspaceMaxSizeMb = options.value.metaspaceMaxSizeMb ?? 512;
  } else {
    options.value.metaspaceSizeMb = null;
    options.value.metaspaceMaxSizeMb = null;
  }
}

function resetToDefaults() {
  options.value = {
    gcStrategy: 'g1',
    alwaysPreTouch: false,
    disableExplicitGc: false,
    stringDeduplication: false,
    perfDisableSharedMem: false,
    metaspaceSizeMb: null,
    metaspaceMaxSizeMb: null,
    heapDumpOnOom: false,
    extraArgs: '',
  };
  metaspaceEnabled.value = false;
}

watch(
  options,
  async () => {
    if (!isInitialized.value) return;
    await saveOptions();
  },
  { deep: true },
);

watch(
  () => [options.value.metaspaceSizeMb, options.value.metaspaceMaxSizeMb],
  () => {
    if (!isInitialized.value) return;
    metaspaceEnabled.value = metaspaceConfigured(options.value);
  },
);

onMounted(async () => {
  await settingsStore.loadSettings();
  await loadOptions();
});
</script>

<template>
  <div class="settings-group">
    <v-card variant="flat" color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-1">
          <v-icon size="20" class="mr-2" color="on-surface-variant">mdi-recycle</v-icon>
          <span class="text-title-medium">垃圾回收器</span>
        </div>
        <p class="text-body-medium text-on-surface-variant mb-4">默认 G1GC 会使用调优参数</p>

        <v-select
          v-model="options.gcStrategy"
          :items="gcOptions"
          item-title="title"
          item-value="value"
          label="GC 策略"
          density="comfortable"
          variant="outlined"
          hide-details
        />

        <p class="text-body-medium text-on-surface-variant mt-4 mb-0">将写入的参数</p>
        <pre class="jvm-code mt-2">{{ gcArgsPreview }}</pre>
      </v-card-text>
    </v-card>

    <v-card variant="flat" color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-3">
          <v-icon size="20" class="mr-2" color="on-surface-variant">mdi-tune-variant</v-icon>
          <span class="text-title-medium">可选优化项</span>
        </div>
        <p class="text-body-medium text-on-surface-variant mb-4">
          以下选项默认全部关闭，以保证与官方启动器一致的稳定性
        </p>

        <div class="option-row">
          <div class="option-text">
            <span class="text-body-medium">预分配堆内存 (AlwaysPreTouch)</span>
            <p class="text-body-small text-on-surface-variant mb-0">
              启动时提交整个堆，减少运行期缺页抖动；会让启动变慢
            </p>
          </div>
          <v-switch
            v-model="options.alwaysPreTouch"
            aria-label="预分配堆内存"
            hide-details
            inset
            density="compact"
            color="primary"
          />
        </div>

        <v-divider class="my-3" />

        <div class="option-row">
          <div class="option-text">
            <span class="text-body-medium">禁用显式 GC (DisableExplicitGC)</span>
            <p class="text-body-small text-on-surface-variant mb-0">
              阻止模组调用 System.gc() 触发的整堆回收；少数模组依赖它释放堆外内存
            </p>
          </div>
          <v-switch
            v-model="options.disableExplicitGc"
            aria-label="禁用显式 GC"
            hide-details
            inset
            density="compact"
            color="primary"
          />
        </div>

        <v-divider class="my-3" />

        <div class="option-row">
          <div class="option-text">
            <span class="text-body-medium">字符串去重</span>
            <p class="text-body-small text-on-surface-variant mb-0">省内存，但会拉长每次 Young GC 的时间</p>
          </div>
          <v-switch
            v-model="options.stringDeduplication"
            aria-label="字符串去重"
            hide-details
            inset
            density="compact"
            color="primary"
          />
        </div>

        <v-divider class="my-3" />

        <div class="option-row">
          <div class="option-text">
            <span class="text-body-medium">关闭性能计数器 (PerfDisableSharedMem)</span>
            <p class="text-body-small text-on-surface-variant mb-0">
              不再写 hsperfdata 共享内存；会失去 jstat 等外部监控能力
            </p>
          </div>
          <v-switch
            v-model="options.perfDisableSharedMem"
            aria-label="关闭性能计数器"
            hide-details
            inset
            density="compact"
            color="primary"
          />
        </div>

        <v-divider class="my-3" />

        <div class="option-row">
          <div class="option-text">
            <span class="text-body-medium">崩溃时导出堆转储</span>
            <p class="text-body-small text-on-surface-variant mb-0">
              内存溢出时把堆写到 ./logs/heapdump.hprof，可能占用数 GB 磁盘
            </p>
          </div>
          <v-switch
            v-model="options.heapDumpOnOom"
            aria-label="崩溃时导出堆转储"
            hide-details
            inset
            density="compact"
            color="primary"
          />
        </div>
      </v-card-text>
    </v-card>

    <v-card variant="flat" color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center justify-space-between mb-1">
          <div class="d-flex align-center">
            <v-icon size="20" class="mr-2" color="on-surface-variant">mdi-memory</v-icon>
            <span class="text-title-medium">Metaspace 元空间</span>
          </div>
          <v-switch
            :model-value="metaspaceEnabled"
            aria-label="设置 Metaspace 大小"
            hide-details
            inset
            density="compact"
            color="primary"
            @update:model-value="toggleMetaspace"
          />
        </div>
        <p class="text-body-medium text-on-surface-variant mb-0">
          存放类与模组的元数据。初始水位抬高可减少加载阶段的扩容；上限用来防止大型整合包无限膨胀
        </p>

        <v-expand-transition>
          <div v-if="metaspaceEnabled" class="mt-4">
            <div class="text-body-medium mb-2">初始大小（MetaspaceSize）</div>
            <div class="d-flex flex-wrap ga-2 mb-3">
              <v-chip
                v-for="mb in metaspacePresets"
                :key="mb"
                :variant="options.metaspaceSizeMb === mb ? 'flat' : 'tonal'"
                :color="options.metaspaceSizeMb === mb ? 'primary' : 'secondary'"
                @click="options.metaspaceSizeMb = mb"
              >
                {{ mb }} MB
              </v-chip>
            </div>
            <v-text-field
              v-model.number="options.metaspaceSizeMb"
              type="number"
              label="初始大小"
              suffix="MB"
              density="compact"
              hide-details
              :min="0"
              hint="超过上限时会自动收敛到上限值"
              style="max-width: 220px"
            />

            <v-divider class="my-4" />

            <div class="text-body-medium mb-2">上限（MaxMetaspaceSize）</div>
            <div class="d-flex flex-wrap ga-2 mb-3">
              <v-chip
                v-for="mb in metaspaceMaxPresets"
                :key="mb"
                :variant="options.metaspaceMaxSizeMb === mb ? 'flat' : 'tonal'"
                :color="options.metaspaceMaxSizeMb === mb ? 'primary' : 'secondary'"
                @click="options.metaspaceMaxSizeMb = mb"
              >
                {{ mb }} MB
              </v-chip>
            </div>
            <v-text-field
              v-model.number="options.metaspaceMaxSizeMb"
              type="number"
              label="上限"
              suffix="MB"
              density="compact"
              hide-details
              :min="0"
              hint="留空或不填表示不限制；只填上限时初始值会自动对齐上限"
              style="max-width: 220px"
            />

            <v-alert v-if="metaspaceTooSmall" color="warning-container" density="compact" class="mt-4">
              <span class="text-body-small text-on-warning-container">
                上限给低了会导致 <code>OutOfMemoryError: Metaspace</code>，大型整合包建议 512 MB 起
              </span>
            </v-alert>
          </div>
        </v-expand-transition>
      </v-card-text>
    </v-card>

    <v-card variant="flat" color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center mb-1">
          <v-icon size="20" class="mr-2" color="on-surface-variant">mdi-console-line</v-icon>
          <span class="text-title-medium">自定义 JVM 参数</span>
        </div>
        <p class="text-body-medium text-on-surface-variant mb-3">
          每行一条或空格分隔，<code>#</code> 开头为注释。这些参数追加在最后，会覆盖上面的同名设置
        </p>

        <v-textarea
          v-model="options.extraArgs"
          variant="outlined"
          density="compact"
          rows="5"
          hide-details
          auto-grow
          :disabled="saving"
          spellcheck="false"
          placeholder="# 例如：使用 ZGC 替代 G1GC&#10;-XX:+UseZGC&#10;-XX:+ZGenerational"
        />
      </v-card-text>
    </v-card>

    <v-card variant="flat" color="surface-container" class="mb-4">
      <v-card-text class="pa-4">
        <div class="d-flex align-center justify-space-between mb-3">
          <div class="d-flex align-center">
            <v-icon size="20" class="mr-2" color="on-surface-variant">mdi-file-document-outline</v-icon>
            <span class="text-title-medium">实际生效的 JVM 参数</span>
          </div>
          <v-btn
            size="small"
            variant="text"
            color="primary"
            :loading="saving"
            @click="
              showPreview = !showPreview;
              refreshPreview();
            "
          >
            {{ showPreview ? '刷新' : '预览' }}
          </v-btn>
        </div>

        <v-expand-transition>
          <div v-if="showPreview">
            <pre v-if="effectiveArgs.length" class="jvm-code jvm-code-block">{{ effectiveArgs.join('\n') }}</pre>
            <p v-else class="text-body-medium text-on-surface-variant mb-0">暂无预览数据</p>
          </div>
        </v-expand-transition>
      </v-card-text>
    </v-card>

    <v-alert color="warning-container" density="compact" class="tip-alert mb-4">
      <template #prepend>
        <v-icon size="20" color="on-warning-container">mdi-alert-outline</v-icon>
      </template>
      <ul class="text-body-small pl-4 mb-0 text-on-warning-container">
        <li>修改会在下次启动游戏时生效</li>
        <li>自定义参数写错可能导致游戏无法启动，改动前先点「预览」确认</li>
      </ul>
    </v-alert>

    <v-btn variant="text" color="secondary" prepend-icon="mdi-restore" @click="resetToDefaults">
      恢复默认（只保留 G1GC）
    </v-btn>
  </div>
</template>

<style scoped>
.option-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.option-text {
  flex: 1 1 auto;
  min-width: 0;
}

.jvm-code {
  display: block;
  padding: 12px;
  border-radius: 8px;
  background: rgb(var(--v-theme-surface-container-high));
  font-family: monospace;
  font-size: 12px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-all;
}

.jvm-code-block {
  max-height: 240px;
  overflow-y: auto;
}
</style>
