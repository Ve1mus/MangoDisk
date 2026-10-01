<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';

import MdEmptyState from '@/components/custom/md-empty-state.vue';
import MdInlineNotice from '@/components/custom/md-inline-notice.vue';
import MdPageShell from '@/components/custom/md-page-shell.vue';
import MdSpinner from '@/components/custom/md-spinner.vue';
import MdIcon from '@/components/icons/md-icon.vue';
import { Button } from '@/components/ui/button';
import type { MemoryFindingCode, MemoryPressure } from '@/lib/models/memory-analysis';
import { ICON_NAMES } from '@/lib/models/ui';
import { ByteSizeService } from '@/lib/services/byte-size-service';
import { consumerShare, memoryFindings } from '@/lib/utils/memory-analysis';
import { useMemoryAnalysisStore } from '@/stores/memory-analysis-store';

import MdMemoryCategoryBar from './components/md-memory-category-bar.vue';
import MdMemoryConsumerRow from './components/md-memory-consumer-row.vue';

const { t } = useI18n({ useScope: 'global' });
const store = useMemoryAnalysisStore();
// Track the selected identity rather than its rank, which changes between analyses.
const expandedId = ref<string | null>(null);

// Literal keys keep every message discoverable by the locale usage check.
const PRESSURE_KEYS: Record<MemoryPressure, string> = {
  normal: 'memoryAnalysis.pressure.normal',
  warning: 'memoryAnalysis.pressure.warning',
  critical: 'memoryAnalysis.pressure.critical',
};
const FINDING_KEYS: Record<MemoryFindingCode, string> = {
  pressureCritical: 'memoryAnalysis.findings.pressureCritical',
  pressureWarning: 'memoryAnalysis.findings.pressureWarning',
  swapInUse: 'memoryAnalysis.findings.swapInUse',
  heavyCompression: 'memoryAnalysis.findings.heavyCompression',
};

const analysis = computed(() => store.analysis);
const findings = computed(() => (analysis.value ? memoryFindings(analysis.value.overview) : []));
const largestBytes = computed(() => Math.max(0, ...(analysis.value?.consumers.map(row => row.bytes) ?? [])));

async function analyze() {
  await store.analyze();
  if (!analysis.value?.consumers.some(row => row.id === expandedId.value)) expandedId.value = null;
}
</script>

<template>
  <MdPageShell :title="t('memoryAnalysis.title')" :subtitle="t('memoryAnalysis.subtitle')">
    <template #actions>
      <Button v-if="analysis" variant="outline" type="button" :disabled="store.analyzing" @click="analyze">
        <MdIcon :name="ICON_NAMES.refresh" :size="16" />
        {{ t('memoryAnalysis.refresh') }}
      </Button>
    </template>

    <div v-if="store.analyzing && !analysis" class="flex items-center gap-2 py-10 text-muted-foreground" role="status">
      <MdSpinner />
      {{ t('memoryAnalysis.analyzing') }}
    </div>

    <MdEmptyState
      v-else-if="!analysis"
      :icon-name="ICON_NAMES.memory"
      :title="t('memoryAnalysis.idleTitle')"
      :description="t('memoryAnalysis.idleDescription')"
    >
      <Button size="lg" type="button" @click="analyze">
        <MdIcon :name="ICON_NAMES.search" :size="17" />
        {{ t('memoryAnalysis.analyze') }}
      </Button>
    </MdEmptyState>

    <template v-else>
      <section class="rounded-lg border border-border p-4" :aria-busy="store.analyzing">
        <div class="flex flex-wrap items-baseline justify-between gap-x-6 gap-y-1 pb-3">
          <p class="m-0 text-lg">
            <strong class="font-semibold tabular-nums">{{
              ByteSizeService.memory(analysis.overview.usedBytes)
            }}</strong>
            <span class="text-muted-foreground">
              / {{ ByteSizeService.memory(analysis.overview.totalBytes) }} · {{ analysis.overview.usedPercent }}%
            </span>
          </p>
          <p class="m-0 flex flex-wrap items-center gap-x-4 text-sm text-muted-foreground">
            <span v-if="analysis.overview.pressure">
              {{ t('memoryAnalysis.pressure.label') }}:
              <strong class="font-medium text-foreground">{{ t(PRESSURE_KEYS[analysis.overview.pressure]) }}</strong>
            </span>
            <span v-if="analysis.overview.swapTotalBytes > 0">
              {{
                t('memoryAnalysis.swap', {
                  used: ByteSizeService.memory(analysis.overview.swapUsedBytes),
                  total: ByteSizeService.memory(analysis.overview.swapTotalBytes),
                })
              }}
            </span>
          </p>
        </div>
        <MdMemoryCategoryBar :overview="analysis.overview" />
      </section>

      <div v-if="findings.length" class="mt-3 flex flex-col gap-2">
        <MdInlineNotice
          v-for="finding in findings"
          :key="finding.code"
          :tone="finding.code === 'pressureCritical' ? 'destructive' : 'warning'"
        >
          {{ t(FINDING_KEYS[finding.code], { size: ByteSizeService.memory(finding.bytes ?? 0) }) }}
        </MdInlineNotice>
      </div>

      <section class="mt-4" :aria-label="t('memoryAnalysis.consumersTitle')">
        <div class="flex items-baseline justify-between gap-3 pb-2">
          <h2 class="m-0 text-sm font-semibold">{{ t('memoryAnalysis.consumersTitle') }}</h2>
          <span class="text-xs text-muted-foreground">
            {{
              t(analysis.metric === 'footprint' ? 'memoryAnalysis.metricFootprint' : 'memoryAnalysis.metricResident')
            }}
          </span>
        </div>
        <MdInlineNotice v-if="analysis.metric === 'footprint'" class="mb-2" tone="info">
          {{ t('memoryAnalysis.footprintHint') }}
        </MdInlineNotice>
        <p v-if="!analysis.consumers.length" class="m-0 text-sm text-muted-foreground">
          {{ t('memoryAnalysis.noConsumers') }}
        </p>
        <ol v-else class="m-0 flex list-none flex-col gap-1.5 p-0">
          <MdMemoryConsumerRow
            v-for="consumer in analysis.consumers"
            :key="consumer.id"
            :consumer="consumer"
            :share="consumerShare(consumer.bytes, largestBytes)"
            :expanded="expandedId === consumer.id"
            @toggle="expandedId = expandedId === consumer.id ? null : consumer.id"
          />
        </ol>
      </section>
    </template>
  </MdPageShell>
</template>
