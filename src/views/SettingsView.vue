<script setup lang="ts">
import MemorySettings from "../components/settings/MemorySettings.vue";
import JavaSettings from "../components/settings/JavaSettings.vue";
import GeneralSettings from "../components/settings/GeneralSettings.vue";
</script>

<template>
  <v-container fluid>
    <v-row>
      <v-col cols="12">
        <h1 class="text-h4 mb-4">设置</h1>
      </v-col>
    </v-row>

    <v-row>
      <v-col cols="12">
        <GeneralSettings class="mb-6" />
      </v-col>
    </v-row>

    <v-row>
      <v-col cols="12" md="6">
        <JavaSettings class="h-100" />
      </v-col>
      <v-col cols="12" md="6">
        <MemorySettings class="h-100" />
      </v-col>
    </v-row>
  </v-container>
</template>
