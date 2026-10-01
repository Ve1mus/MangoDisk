<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';

import MdIconAction from '@/components/custom/md-icon-action.vue';
import MdNativeFileIcon from '@/components/custom/md-native-file-icon.vue';
import MdIcon from '@/components/icons/md-icon.vue';
import { Button } from '@/components/ui/button';
import type { MemoryConsumer } from '@/lib/models/memory-analysis';
import type { ApplicationQuitStatus } from '@/lib/models/resident';
import { LOG_DOMAINS, LOG_EVENTS } from '@/lib/models/telemetry';
import { ICON_NAMES } from '@/lib/models/ui';
import { ByteSizeService } from '@/lib/services/byte-size-service';
import { FileManagerService } from '@/lib/services/file-manager-service';
import { LoggerService } from '@/lib/services/logger-service';
import { ResidentService } from '@/lib/services/resident-service';
import * as FormatUtils from '@/lib/utils/format';

const props = defineProps<{ consumer: MemoryConsumer; share: number; expanded: boolean }>();
const emit = defineEmits<{ toggle: [] }>();
const { t } = useI18n({ useScope: 'global' });

const quitting = ref(false);
const quitStatus = ref<ApplicationQuitStatus | 'failed' | null>(null);
const revealFailed = ref(false);
const QUIT_MESSAGE_KEYS = {
  requested: 'monitoring.quitRequested',
  unavailable: 'monitoring.quitUnavailable',
  unsupported: 'monitoring.quitUnsupported',
  failed: 'monitoring.quitFailed',
} as const;

const hiddenProcessCount = computed(() => props.consumer.processCount - props.consumer.processes.length);

// A new ranking can reuse this row for another state of the same application.
watch(
  () => props.expanded,
  () => {
    quitStatus.value = null;
    revealFailed.value = false;
  }
);

async function quit() {
  if (!props.consumer.canQuit || quitting.value) return;
  quitting.value = true;
  quitStatus.value = null;
  try {
    // The backend resolves this identity again; no PID or path is supplied by the UI.
    quitStatus.value = await ResidentService.quitApplication(props.consumer.id);
  } catch (error) {
    quitStatus.value = 'failed';
    LoggerService.warn(LOG_DOMAINS.memoryAnalysis, LOG_EVENTS.memoryApplicationQuitFailed, {
      name: props.consumer.name,
      error,
    });
  } finally {
    quitting.value = false;
  }
}

async function reveal() {
  const path = props.consumer.iconPath;
  if (!path) return;
  revealFailed.value = false;
  try {
    await FileManagerService.reveal(path);
  } catch (error) {
    revealFailed.value = true;
    LoggerService.warn(LOG_DOMAINS.memoryAnalysis, LOG_EVENTS.memoryApplicationRevealFailed, { path, error });
  }
}
</script>

<template>
  <li class="overflow-hidden rounded-lg border border-border">
    <button
      type="button"
      class="relative flex w-full items-center gap-3 px-3 py-2 text-left focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none focus-visible:ring-inset"
      :aria-expanded="expanded"
      :aria-controls="`memory-consumer-${consumer.id}`"
      @click="emit('toggle')"
    >
      <span class="absolute inset-y-0 left-0 bg-primary/10" aria-hidden="true" :style="{ width: `${share}%` }" />
      <span class="relative grid h-7 w-7 shrink-0 place-items-center">
        <MdNativeFileIcon
          v-if="consumer.iconPath"
          :path="consumer.iconPath"
          :name="consumer.name"
          :directory="consumer.isBundle"
          directory-mode="path"
          compact
        />
        <MdIcon v-else :name="ICON_NAMES.application" :size="20" />
      </span>
      <span class="relative min-w-0 flex-1 truncate font-medium">{{ consumer.name }}</span>
      <span class="relative shrink-0 text-xs text-muted-foreground">
        {{ t('monitoring.processCount', { count: consumer.processCount }) }}
      </span>
      <strong class="relative shrink-0 text-sm tabular-nums">
        <template v-if="consumer.approximate">≈ </template>{{ ByteSizeService.memory(consumer.bytes) }}
      </strong>
      <MdIcon
        class="relative shrink-0 text-muted-foreground"
        :name="expanded ? ICON_NAMES.chevronUp : ICON_NAMES.chevronDown"
        :size="14"
      />
    </button>

    <div v-if="expanded" :id="`memory-consumer-${consumer.id}`" class="border-t border-border px-3 py-2.5">
      <ul class="m-0 flex list-none flex-col gap-1 p-0">
        <li v-for="process in consumer.processes" :key="process.pid" class="flex items-baseline gap-3 text-xs">
          <span class="min-w-0 flex-1 truncate">{{ process.name }}</span>
          <span class="shrink-0 text-muted-foreground">{{ t('memoryAnalysis.pid', { pid: process.pid }) }}</span>
          <span class="w-20 shrink-0 text-right tabular-nums">
            <template v-if="process.approximate">≈ </template>{{ ByteSizeService.memory(process.bytes) }}
          </span>
        </li>
      </ul>
      <p v-if="hiddenProcessCount > 0" class="m-0 mt-1 text-xs text-muted-foreground">
        {{ t('memoryAnalysis.moreProcesses', { count: FormatUtils.integer(hiddenProcessCount) }) }}
      </p>
      <p v-if="consumer.approximate" class="m-0 mt-1 text-xs text-muted-foreground">
        {{ t('memoryAnalysis.approximateHint') }}
      </p>
      <p v-if="consumer.iconPath" class="m-0 mt-2 truncate text-xs text-muted-foreground">{{ consumer.iconPath }}</p>
      <div class="mt-2 flex flex-wrap items-center gap-2">
        <MdIconAction v-if="consumer.iconPath" :label="t('common.showInFileManager')" @click="reveal">
          <MdIcon :name="ICON_NAMES.folderOpen" :size="16" />
        </MdIconAction>
        <Button v-if="consumer.canQuit" variant="outline" type="button" size="sm" :disabled="quitting" @click="quit">
          {{ t(quitting ? 'monitoring.quittingApplication' : 'monitoring.quitApplication') }}
        </Button>
        <span v-if="quitStatus" class="text-xs text-muted-foreground" role="status">
          {{ t(QUIT_MESSAGE_KEYS[quitStatus]) }}
        </span>
        <span v-if="revealFailed" class="text-xs text-destructive" role="status">{{
          t('monitoring.revealFailed')
        }}</span>
      </div>
    </div>
  </li>
</template>
