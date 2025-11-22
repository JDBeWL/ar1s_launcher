import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';

export interface ModrinthModpack {
    slug: string;
    title: string;
    author: string;
    downloads: number;
    game_versions: string[];
    loaders: string[];
    description: string;
    icon_url?: string;
    date_created: string;
    date_modified: string;
    latest_version: string;
    categories: string[];
}

export interface ModrinthSearchResult {
    hits: ModrinthModpack[];
    total_hits: number;
}

export function useModrinth() {
    const modpackSearchQuery = ref("");
    const selectedGameVersion = ref<string | null>(null);
    const selectedLoader = ref<string | null>(null);
    const selectedCategory = ref<string | null>(null);
    const sortBy = ref("relevance");
    const modpacks = ref<ModrinthModpack[]>([]);
    const loadingModpacks = ref(false);
    const modpackCurrentPage = ref(1);
    const modpackItemsPerPage = 12;
    const modpackTotalHits = ref(0);
    const modpackInitialLoad = ref(true);

    const modpackTotalPages = computed(() => Math.ceil(modpackTotalHits.value / modpackItemsPerPage));

    const gameVersions = ref<string[]>([]);
    const loaders = ["fabric", "forge", "quilt", "neoforge"];
    const categories = [
        "adventure", "cursed", "decoration", "economy", "equipment",
        "food", "game-mechanics", "library", "magic", "optimization",
        "social", "storage", "technology", "transportation", "utility", "worldgen"
    ];
    const modpackSortOptions = [
        { title: "相关性", value: "relevance" },
        { title: "下载量", value: "downloads" },
        { title: "关注度", value: "follows" },
        { title: "最新", value: "newest" },
        { title: "更新", value: "updated" }
    ];

    async function fetchGameVersions() {
        try {
            const manifest = await invoke("get_versions");
            gameVersions.value = (manifest as any).versions
                .filter((v: any) => v.type === "release")
                .map((v: any) => v.id)
                .sort((a: string, b: string) => {
                    const aParts = a.split('.').map(Number);
                    const bParts = b.split('.').map(Number);
                    for (let i = 0; i < Math.max(aParts.length, bParts.length); i++) {
                        const aPart = aParts[i] || 0;
                        const bPart = bParts[i] || 0;
                        if (aPart !== bPart) return bPart - aPart;
                    }
                    return 0;
                });
        } catch (error) {
            console.error("Failed to fetch game versions for filter:", error);
        }
    }

    async function searchModpacks() {
        loadingModpacks.value = true;
        try {
            const offset = (modpackCurrentPage.value - 1) * modpackItemsPerPage;

            const result = await invoke("search_modrinth_modpacks", {
                query: modpackSearchQuery.value || undefined,
                gameVersions: selectedGameVersion.value ? [selectedGameVersion.value] : undefined,
                loaders: selectedLoader.value ? [selectedLoader.value] : undefined,
                categories: selectedCategory.value ? [selectedCategory.value] : undefined,
                limit: modpackItemsPerPage,
                offset: offset,
                sortBy: sortBy.value,
            }) as ModrinthSearchResult;

            modpacks.value = result.hits || [];
            modpackTotalHits.value = result.total_hits || 0;
            modpackInitialLoad.value = false;
        } catch (error) {
            console.error("搜索整合包失败:", error);
            modpacks.value = [];
            modpackTotalHits.value = 0;
            modpackInitialLoad.value = false;
        } finally {
            loadingModpacks.value = false;
        }
    }

    async function onModpackPageChange(page: number) {
        modpackCurrentPage.value = page;
        await searchModpacks();
    }

    return {
        modpackSearchQuery,
        selectedGameVersion,
        selectedLoader,
        selectedCategory,
        sortBy,
        modpacks,
        loadingModpacks,
        modpackCurrentPage,
        modpackTotalHits,
        modpackTotalPages,
        modpackInitialLoad,
        gameVersions,
        loaders,
        categories,
        modpackSortOptions,
        fetchGameVersions,
        searchModpacks,
        onModpackPageChange
    };
}
