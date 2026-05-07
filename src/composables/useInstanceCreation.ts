import { ref, computed, watch } from 'vue';
import { api } from '../services';
import { useNotificationStore } from '../stores/notificationStore';
import { useDebounceFn } from './useDebounce';
import { useEventSubscription } from './useEventSubscription';
import { getErrorMessage } from '../utils/format';
import { logError } from '../utils/logger';
import { sortVersionsByReleaseTime } from '../utils/format';
import type { 
    MinecraftVersion, 
    InstallProgressPayload, 
    ForgeVersion,
    LoaderVersionInfo,
    AvailableLoaders,
    ModLoaderType,
    LoaderPayload,
} from '../types/events';

export function useInstanceCreation() {
    const versions = ref<MinecraftVersion[]>([]);
    const loadingVersions = ref(false);
    const selectedVersion = ref<MinecraftVersion | null>(null);
    const searchVersion = ref("");
    const versionTypeFilter = ref("release");
    const sortOrder = ref("newest");

    const instanceName = ref("");
    const instanceNameError = ref<string | null>(null);
    const installing = ref(false);
    const showProgress = ref(false);
    const progressValue = ref(0);
    const progressIndeterminate = ref(false);
    const progressText = ref("");

    // 加载器相关
    const availableLoaders = ref<AvailableLoaders | null>(null);
    const loadingAvailableLoaders = ref(false);
    const selectedModLoaderType = ref<ModLoaderType>("None");
    const modLoaderVersions = ref<(ForgeVersion | LoaderVersionInfo)[]>([]);
    const loadingModLoaderVersions = ref(false);
    const selectedModLoaderVersion = ref<ForgeVersion | LoaderVersionInfo | null>(null);

    // 动态计算可用的加载器类型
    const modLoaderTypes = computed<{ title: string; value: ModLoaderType; disabled: boolean }[]>(() => {
        const loaders = availableLoaders.value;
        return [
            { title: '无', value: 'None' as ModLoaderType, disabled: false },
            { title: 'Forge', value: 'Forge' as ModLoaderType, disabled: !loaders?.forge },
            { title: 'Fabric', value: 'Fabric' as ModLoaderType, disabled: !loaders?.fabric },
            { title: 'Quilt', value: 'Quilt' as ModLoaderType, disabled: !loaders?.quilt },
            { title: 'NeoForge', value: 'NeoForge' as ModLoaderType, disabled: !loaders?.neoforge },
        ];
    });

    // 先过滤版本（只依赖 versions, versionTypeFilter, searchVersion）
    const filteredVersionsUnsorted = computed(() => {
        return versions.value.filter((version) => {
            const typeMatch =
                versionTypeFilter.value === "all" ||
                version.type === versionTypeFilter.value;
            const searchMatch =
                !searchVersion.value ||
                version.id.toLowerCase().includes(searchVersion.value.toLowerCase());
            return typeMatch && searchMatch;
        });
    });

    // 再排序（只依赖 filteredVersionsUnsorted 和 sortOrder）
    const filteredVersions = computed(() => {
        if (sortOrder.value === "newest" || sortOrder.value === "oldest") {
            return sortVersionsByReleaseTime(filteredVersionsUnsorted.value, sortOrder.value);
        }
        return filteredVersionsUnsorted.value;
    });

    const defaultInstanceName = computed(() => {
        if (selectedVersion.value) {
            if (selectedModLoaderType.value && selectedModLoaderType.value !== 'None') {
                return `${selectedVersion.value.id}-${selectedModLoaderType.value}`;
            }
            return selectedVersion.value.id;
        }
        return "";
    });

    async function fetchVersions() {
        loadingVersions.value = true;
        try {
            const manifest = await api.version.getVersions();
            // 保留原始 releaseTime (ISO 8601) 用于排序，显示时再格式化
            versions.value = manifest.versions;
        } catch (error) {
            logError("Failed to fetch versions", error, 'useInstanceCreation');
        } finally {
            loadingVersions.value = false;
        }
    }

    async function fetchAvailableLoaders() {
        if (!selectedVersion.value) {
            availableLoaders.value = null;
            return;
        }

        loadingAvailableLoaders.value = true;
        try {
            const loaders = await api.loader.getAvailableLoaders(selectedVersion.value.id);
            availableLoaders.value = loaders;
            
            // 如果当前选择的加载器不可用，重置为 None
            if (selectedModLoaderType.value !== 'None') {
                const loaderKey = selectedModLoaderType.value.toLowerCase() as keyof AvailableLoaders;
                if (!loaders[loaderKey]) {
                    selectedModLoaderType.value = 'None';
                }
            }
        } catch (error) {
            logError("Failed to fetch available loaders", error, 'useInstanceCreation');
            availableLoaders.value = null;
        } finally {
            loadingAvailableLoaders.value = false;
        }
    }

    async function fetchModLoaderVersions() {
        if (
            !selectedVersion.value ||
            !selectedModLoaderType.value ||
            selectedModLoaderType.value === "None"
        ) {
            modLoaderVersions.value = [];
            selectedModLoaderVersion.value = null;
            return;
        }

        loadingModLoaderVersions.value = true;
        selectedModLoaderVersion.value = null;

        try {
            const mcVersion = selectedVersion.value.id;
            let result: (ForgeVersion | LoaderVersionInfo)[] = [];

            switch (selectedModLoaderType.value) {
                case "Forge":
                    result = await api.loader.getForgeVersions(mcVersion);
                    break;
                case "Fabric":
                    result = await api.loader.getFabricVersions(mcVersion);
                    break;
                case "Quilt":
                    result = await api.loader.getQuiltVersions(mcVersion);
                    break;
                case "NeoForge":
                    result = await api.loader.getNeoForgeVersions(mcVersion);
                    break;
            }

            modLoaderVersions.value = result;

            if (result.length > 0) {
                selectedModLoaderVersion.value = result[0];
            }
        } catch (error) {
            logError(
                `Failed to fetch ${selectedModLoaderType.value} versions`,
                error,
                'useInstanceCreation'
            );
            modLoaderVersions.value = [];
        } finally {
            loadingModLoaderVersions.value = false;
        }
    }

    // 当选择的 MC 版本变化时，获取可用加载器
    watch(selectedVersion, () => {
        selectedModLoaderType.value = 'None';
        modLoaderVersions.value = [];
        selectedModLoaderVersion.value = null;
        fetchAvailableLoaders();
    });

    // 当选择的加载器类型变化时，获取加载器版本
    watch(selectedModLoaderType, () => {
        fetchModLoaderVersions();
    });

    // 验证实例名称
    async function validateInstanceName(name: string): Promise<boolean> {
        if (!name) {
            instanceNameError.value = null;
            return true; // 空名称会使用默认名称
        }
        
        try {
            const result = await api.instance.validateInstanceName(name);
            if (!result.is_valid) {
                instanceNameError.value = result.error_message;
                return false;
            }
            instanceNameError.value = null;
            return true;
        } catch (error) {
            logError('Failed to validate instance name', error, 'useInstanceCreation');
            instanceNameError.value = '验证失败';
            return false;
        }
    }

    // 当实例名称变化时验证（防抖）
    const debouncedValidate = useDebounceFn((name: string) => {
        validateInstanceName(name);
    }, 300);

    watch(instanceName, (newName) => {
        debouncedValidate.call(newName);
    });

    // 使用统一的事件订阅管理
    const installProgressSub = useEventSubscription<InstallProgressPayload>(
        "instance-install-progress",
        (event) => {
            const progressData = event.payload;
            progressValue.value = progressData.progress;
            progressText.value = progressData.message;
            progressIndeterminate.value = progressData.indeterminate;
        }
    );

    async function createInstance() {
        const notificationStore = useNotificationStore();
        
        if (!selectedVersion.value) {
            notificationStore.warning('请先选择一个Minecraft版本');
            return;
        }

        const finalInstanceName = instanceName.value || defaultInstanceName.value;
        if (!finalInstanceName) {
            notificationStore.warning('实例名称不能为空');
            return;
        }

        // 验证实例名称
        const isValid = await validateInstanceName(finalInstanceName);
        if (!isValid) {
            notificationStore.warning('实例名称无效', instanceNameError.value || '请检查实例名称');
            return;
        }

        installing.value = true;
        showProgress.value = true;
        progressValue.value = 0;
        progressIndeterminate.value = true;
        progressText.value = "准备安装...";

        await installProgressSub.subscribe();

        try {
            const payload: {
                newInstanceName: string;
                baseVersionId: string;
                loader?: LoaderPayload;
            } = {
                newInstanceName: finalInstanceName,
                baseVersionId: selectedVersion.value.id,
            };

            // 根据加载器类型设置对应的版本
            if (selectedModLoaderVersion.value && selectedModLoaderType.value !== 'None') {
                const mcVersion = selectedVersion.value.id;
                // ForgeVersion 和 LoaderVersionInfo 都有 version 字段
                const loaderVersion = 'version' in selectedModLoaderVersion.value 
                    ? selectedModLoaderVersion.value.version 
                    : String(selectedModLoaderVersion.value);
                
                switch (selectedModLoaderType.value) {
                    case 'Forge':
                        payload.loader = {
                            type: 'forge',
                            mc_version: mcVersion,
                            loader_version: loaderVersion,
                        };
                        break;
                    case 'Fabric':
                        payload.loader = {
                            type: 'fabric',
                            mc_version: mcVersion,
                            loader_version: loaderVersion,
                        };
                        break;
                    case 'Quilt':
                        payload.loader = {
                            type: 'quilt',
                            mc_version: mcVersion,
                            loader_version: loaderVersion,
                        };
                        break;
                    case 'NeoForge':
                        payload.loader = {
                            type: 'neoforge',
                            mc_version: mcVersion,
                            loader_version: loaderVersion,
                        };
                        break;
                }
            }

            await api.instance.createInstance(
                finalInstanceName,
                selectedVersion.value.id,
                payload.loader
            );

            notificationStore.success('创建成功', `实例 '${finalInstanceName}' 已创建`);

            showProgress.value = false;
            installing.value = false;
        } catch (error) {
            logError("Failed to create instance", error, 'useInstanceCreation');
            progressText.value = "安装失败！";
            progressIndeterminate.value = false;
            installing.value = false;

            await new Promise((resolve) => setTimeout(resolve, 1000));
            showProgress.value = false;

            notificationStore.error('创建实例失败', getErrorMessage(error), true);
        } finally {
            installProgressSub.unsubscribe();
        }
    }

    return {
        versions,
        loadingVersions,
        selectedVersion,
        searchVersion,
        versionTypeFilter,
        sortOrder,
        filteredVersions,
        instanceName,
        instanceNameError,
        defaultInstanceName,
        installing,
        showProgress,
        progressValue,
        progressIndeterminate,
        progressText,
        availableLoaders,
        loadingAvailableLoaders,
        modLoaderTypes,
        selectedModLoaderType,
        modLoaderVersions,
        loadingModLoaderVersions,
        selectedModLoaderVersion,
        fetchVersions,
        fetchAvailableLoaders,
        fetchModLoaderVersions,
        validateInstanceName,
        createInstance
    };
}
