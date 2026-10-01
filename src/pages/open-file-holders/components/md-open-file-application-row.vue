<script setup lang="ts">
import { useI18n } from 'vue-i18n';

import MdIconAction from '@/components/custom/md-icon-action.vue';
import MdNativeFileIcon from '@/components/custom/md-native-file-icon.vue';
import MdStatusBadge from '@/components/custom/md-status-badge.vue';
import MdIcon from '@/components/icons/md-icon.vue';
import { Button } from '@/components/ui/button';
import { ICON_NAMES } from '@/lib/models/ui';
import * as FormatUtils from '@/lib/utils/format';
import { holderIsHelper, type OpenFileApplication } from '@/lib/utils/open-file-holders';

const props = defineProps<{ application: OpenFileApplication; expanded: boolean; busy: boolean }>();
const emit = defineEmits<{ toggle: []; reveal: [path: string]; close: [mode: 'graceful' | 'force'] }>();
const { t } = useI18n({ useScope: 'global' });

const revealPath = () => props.application.applicationPath ?? props.application.executablePaths[0] ?? null;
</script>

<template>
  <li class="overflow-hidden rounded-lg border border-border">
    <div class="flex items-center gap-3 px-3 py-2">
      <button
        type="button"
        class="flex min-w-0 flex-1 items-center gap-3 rounded-md text-left focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
        :aria-expanded="expanded"
        :aria-controls="`open-files-${application.key}`"
        @click="emit('toggle')"
      >
        <MdIcon
          class="shrink-0 text-muted-foreground"
          :name="expanded ? ICON_NAMES.chevronUp : ICON_NAMES.chevronDown"
          :size="14"
        />
        <span class="grid h-7 w-7 shrink-0 place-items-center">
          <MdNativeFileIcon
            v-if="application.applicationPath"
            :path="application.applicationPath"
            :name="application.name"
            directory
            directory-mode="path"
            compact
          />
          <MdIcon v-else :name="ICON_NAMES.application" :size="20" />
        </span>
        <span class="flex min-w-0 flex-1 flex-wrap items-center gap-2">
          <span class="font-medium">{{ application.name }}</span>
          <MdStatusBadge size="compact">
            {{ t('openFileHolders.processCount', { count: FormatUtils.integer(application.holders.length) }) }}
          </MdStatusBadge>
          <MdStatusBadge size="compact">
            {{ t('openFileHolders.openFiles', { count: FormatUtils.integer(application.openFileCount) }) }}
          </MdStatusBadge>
        </span>
      </button>
      <MdIconAction v-if="revealPath()" :label="t('common.showInFileManager')" @click="emit('reveal', revealPath()!)">
        <MdIcon :name="ICON_NAMES.folderOpen" :size="16" />
      </MdIconAction>
      <Button
        variant="outline"
        type="button"
        size="sm"
        :disabled="busy || !application.executablePaths.length"
        @click="emit('close', 'graceful')"
      >
        {{ t('openFileHolders.close') }}
      </Button>
      <Button
        variant="outline"
        type="button"
        size="sm"
        :disabled="busy || !application.executablePaths.length"
        @click="emit('close', 'force')"
      >
        {{ t('openFileHolders.forceClose') }}
      </Button>
    </div>

    <ul v-if="expanded" :id="`open-files-${application.key}`" class="m-0 list-none border-t border-border p-0">
      <li
        v-for="holder in application.holders"
        :key="holder.pid"
        class="px-3 py-2 not-last:border-b not-last:border-border"
      >
        <div class="flex flex-wrap items-center gap-2">
          <span class="text-sm font-medium">{{ holder.command }}</span>
          <MdStatusBadge v-if="holderIsHelper(holder)" size="compact">{{
            t('openFileHolders.helperBadge')
          }}</MdStatusBadge>
          <MdStatusBadge size="compact">{{ t('openFileHolders.pid', { pid: holder.pid }) }}</MdStatusBadge>
          <MdStatusBadge size="compact">
            {{ t('openFileHolders.openFiles', { count: FormatUtils.integer(holder.openFileCount) }) }}
          </MdStatusBadge>
        </div>
        <p v-for="sample in holder.samplePaths" :key="sample" class="m-0 mt-0.5 truncate text-xs text-muted-foreground">
          {{ sample }}
        </p>
        <p v-if="holder.openFileCount > holder.samplePaths.length" class="m-0 mt-0.5 text-xs text-muted-foreground">
          {{
            t('openFileHolders.moreFiles', {
              count: FormatUtils.integer(holder.openFileCount - holder.samplePaths.length),
            })
          }}
        </p>
      </li>
    </ul>
  </li>
</template>
