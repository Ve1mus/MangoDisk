<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';

import type { MemoryAnalysisOverview, MemorySegmentId } from '@/lib/models/memory-analysis';
import { ByteSizeService } from '@/lib/services/byte-size-service';
import { memorySegments } from '@/lib/utils/memory-analysis';

const props = defineProps<{ overview: MemoryAnalysisOverview }>();
const { t } = useI18n({ useScope: 'global' });

// Literal keys keep every message discoverable by the locale usage check.
const LABEL_KEYS: Record<MemorySegmentId, string> = {
  application: 'memoryAnalysis.categories.application',
  wired: 'memoryAnalysis.categories.wired',
  compressed: 'memoryAnalysis.categories.compressed',
  cached: 'memoryAnalysis.categories.cached',
  free: 'memoryAnalysis.categories.free',
};
const HINT_KEYS: Record<MemorySegmentId, string> = {
  application: 'memoryAnalysis.categoryHints.application',
  wired: 'memoryAnalysis.categoryHints.wired',
  compressed: 'memoryAnalysis.categoryHints.compressed',
  cached: 'memoryAnalysis.categoryHints.cached',
  free: 'memoryAnalysis.categoryHints.free',
};
const COLOR_CLASSES: Record<MemorySegmentId, string> = {
  application: 'bg-chart-1',
  wired: 'bg-chart-2',
  compressed: 'bg-chart-3',
  cached: 'bg-chart-4',
  free: 'bg-muted-foreground/30',
};

const segments = computed(() => memorySegments(props.overview));
</script>

<template>
  <div v-if="segments">
    <div class="flex h-3 overflow-hidden rounded-full bg-muted" role="img" :aria-label="t('memoryAnalysis.barLabel')">
      <span
        v-for="segment in segments"
        :key="segment.id"
        class="h-full"
        :class="COLOR_CLASSES[segment.id]"
        :style="{ width: `${segment.percent}%` }"
      />
    </div>
    <ul class="m-0 mt-3 grid list-none grid-cols-1 gap-x-6 gap-y-2 p-0 sm:grid-cols-2">
      <li v-for="segment in segments" :key="segment.id" class="flex items-start gap-2">
        <span class="mt-1 h-2.5 w-2.5 shrink-0 rounded-full" :class="COLOR_CLASSES[segment.id]" aria-hidden="true" />
        <span class="min-w-0 flex-1">
          <span class="flex items-baseline justify-between gap-2">
            <span class="text-sm font-medium">{{ t(LABEL_KEYS[segment.id]) }}</span>
            <span class="text-sm tabular-nums">{{ ByteSizeService.memory(segment.bytes) }}</span>
          </span>
          <span class="block text-xs text-muted-foreground">{{ t(HINT_KEYS[segment.id]) }}</span>
        </span>
      </li>
    </ul>
  </div>
</template>
