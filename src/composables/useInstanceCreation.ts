import { ref, computed, onUnmounted, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { useNotificationStore } from '../stores/notificationStore';
import type { 
    MinecraftVersion, 
    VersionManifest, 
    InstallProgressPayload, 
    ForgeVersion,
    LoaderVersionInfo,
    AvailableLoaders,
    ModLoaderType,
    LoaderPayload
} from '../types/events';

export function useInstanceCreation() {
    const versions = ref<MinecraftVersion[]>([]);
    const loadingVersions = ref(false);
    const selectedVersion = ref<MinecraftVersion | null>(null);
    const searchVersion = ref("");
    const versionTypeFilter = ref("release");
    const sortOrder = ref("newest");

    const instanceName = ref("");
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

    const filteredVersions = computed(() => {
        let filtered = versions.value.filter((version) => {
            const typeMatch =
                versionTypeFilter.value === "all" ||
                version.type === versionTypeFilter.value;
            const searchMatch =
                !searchVersion.value ||
                version.id.toLowerCase().includes(searchVersion.value.toLowerCase());
            return typeMatch && searchMatch;
        });

        if (sortOrder.value === "newest") {
            filtered.sort(
                (a, b) =>
                    new Date(b.releaseTime).getTime() - new Date(a.releaseTime).getTime()
            );
        } else if (sortOrder.value === "oldest") {
            filtered.sort(
                (a, b) =>
                    new Date(a.releaseTime).getTime() - new Date(b.releaseTime).getTime()
            );
        }

        return filtered;
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
            const manifest = await invoke<VersionManifest>("get_versions");
            versions.value = manifest.versions.map((v) => ({
                ...v,
                releaseTime: new Date(v.releaseTime).toLocaleString(),
            }));
        } catch (error) {
            console.error("Failed to fetch versions:", error);
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
            const loaders = await invoke<AvailableLoaders>("get_available_loaders", {
                minecraftVersion: selectedVersion.value.id,
            });
            availableLoaders.value = loaders;
            
            // 如果当前选择的加载器不可用，重置为 None
            if (selectedModLoaderType.value !== 'None') {
                const loaderKey = selectedModLoaderType.value.toLowerCase() as keyof AvailableLoaders;
                if (!loaders[loaderKey]) {
                    selectedModLoaderType.value = 'None';
                }
            }
        } catch (error) {
            console.error("Failed to fetch available loaders:", error);
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
                    result = await invoke<ForgeVersion[]>("get_forge_versions", {
                        minecraftVersion: mcVersion,
                    });
                    break;
                case "Fabric":
                    result = await invoke<LoaderVersionInfo[]>("get_fabric_versions", {
                        minecraftVersion: mcVersion,
                    });
                    break;
                case "Quilt":
                    result = await invoke<LoaderVersionInfo[]>("get_quilt_versions", {
                        minecraftVersion: mcVersion,
                    });
                    break;
                case "NeoForge":
                    result = await invoke<LoaderVersionInfo[]>("get_neoforge_versions", {
                        minecraftVersion: mcVersion,
                    });
                    break;
            }

            modLoaderVersions.value = result;

            if (result.length > 0) {
                selectedModLoaderVersion.value = result[0];
            }
        } catch (error) {
            console.error(
                `Failed to fetch ${selectedModLoaderType.value} versions:`,
                error
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

    let unlistenProgress: UnlistenFn | null = null;

    function cleanup() {
        if (unlistenProgress) {
            unlistenProgress();
            unlistenProgress = null;
        }
    }

    onUnmounted(cleanup);

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

        installing.value = true;
        showProgress.value = true;
        progressValue.value = 0;
        progressIndeterminate.value = true;
        progressText.value = "准备安装...";

        try {
            unlistenProgress = await listen<InstallProgressPayload>(
                "instance-install-progress",
                (event) => {
                    const progressData = event.payload;
                    progressValue.value = progressData.progress;
                    progressText.value = progressData.message;
                    progressIndeterminate.value = progressData.indeterminate;
                }
            );

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
                const loaderVersion = (selectedModLoaderVersion.value as any).version;
                
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

            await invoke("create_instance", { ...payload });

            notificationStore.success('创建成功', `实例 '${finalInstanceName}' 已创建`);

            showProgress.value = false;
            installing.value = false;
        } catch (error) {
            console.error("Failed to create instance:", error);
            progressText.value = "安装失败！";
            progressIndeterminate.value = false;
            installing.value = false;

            await new Promise((resolve) => setTimeout(resolve, 1000));
            showProgress.value = false;

            const errorMessage = error instanceof Error ? error.message : String(error);
            notificationStore.error('创建实例失败', errorMessage, true);
        } finally {
            cleanup();
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
        createInstance
    };
}
