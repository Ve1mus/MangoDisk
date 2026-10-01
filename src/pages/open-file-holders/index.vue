<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';

import MdConfirmDialog from '@/components/custom/md-confirm-dialog.vue';
import MdEmptyState from '@/components/custom/md-empty-state.vue';
import MdInlineNotice from '@/components/custom/md-inline-notice.vue';
import MdPageShell from '@/components/custom/md-page-shell.vue';
import MdSpinner from '@/components/custom/md-spinner.vue';
import MdIcon from '@/components/icons/md-icon.vue';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import type { ApplicationCloseMode } from '@/lib/models/application-close';
import { ICON_NAMES } from '@/lib/models/ui';
import { FileManagerService } from '@/lib/services/file-manager-service';
import { FolderSelectionService } from '@/lib/services/folder-selection-service';
import * as FormatUtils from '@/lib/utils/format';
import { groupHoldersByApplication, type OpenFileApplication } from '@/lib/utils/open-file-holders';
import { useAppStore } from '@/stores/app-store';
import { useOpenFileHoldersStore } from '@/stores/open-file-holders-store';

import MdOpenFileApplicationRow from './components/md-open-file-application-row.vue';

const { t } = useI18n({ useScope: 'global' });
const store = useOpenFileHoldersStore();
const app = useAppStore();

const pending = ref<{ application: OpenFileApplication; mode: ApplicationCloseMode } | null>(null);
const expanded = ref<ReadonlySet<string>>(new Set());
const confirmOpen = computed({
  get: () => pending.value !== null,
  set: open => {
    if (!open) pending.value = null;
  },
});
const pendingName = computed(() => pending.value?.application.name ?? '');
const forcing = computed(() => pending.value?.mode === 'force');
const applications = computed(() => groupHoldersByApplication(store.result?.holders ?? []));
const totals = computed(() => ({
  applications: applications.value.length,
  processes: store.result?.holders.length ?? 0,
  files: applications.value.reduce((total, application) => total + application.openFileCount, 0),
}));

async function chooseFolder() {
  try {
    const [selected] = await FolderSelectionService.select(false, t('openFileHolders.chooseFolder'));
    if (selected) await store.find(selected);
  } catch (error) {
    app.reportError(error);
  }
}

async function confirmClose() {
  const request = pending.value;
  pending.value = null;
  if (request) await store.close(request.application, request.mode);
}

function toggle(key: string) {
  const next = new Set(expanded.value);
  if (!next.delete(key)) next.add(key);
  expanded.value = next;
}

async function reveal(path: string) {
  try {
    await FileManagerService.reveal(path);
  } catch (error) {
    app.reportError(error);
  }
}
</script>

<template>
  <MdPageShell :title="t('openFileHolders.title')" :subtitle="t('openFileHolders.subtitle')">
    <form class="flex items-center gap-2 pb-3" @submit.prevent="store.find(store.path)">
      <Input
        v-model="store.path"
        class="min-w-0 flex-1"
        :placeholder="t('openFileHolders.pathPlaceholder')"
        :aria-label="t('openFileHolders.pathLabel')"
        spellcheck="false"
        autocomplete="off"
      />
      <Button variant="outline" type="button" :disabled="store.busy" @click="chooseFolder">
        <MdIcon :name="ICON_NAMES.folderOpen" :size="16" />
        {{ t('openFileHolders.chooseFolder') }}
      </Button>
      <Button type="submit" :disabled="store.busy">
        <MdIcon :name="store.result ? ICON_NAMES.refresh : ICON_NAMES.search" :size="16" />
        {{ t(store.result ? 'openFileHolders.searchAgain' : 'openFileHolders.find') }}
      </Button>
    </form>

    <MdInlineNotice v-if="store.closeOutcome === 'closed'" class="mb-3" tone="success" role="status">
      {{ t('openFileHolders.closed') }}
    </MdInlineNotice>
    <MdInlineNotice v-else-if="store.closeOutcome === 'stillRunning'" class="mb-3" tone="warning" role="status">
      {{ t('openFileHolders.stillRunning') }}
    </MdInlineNotice>

    <div
      v-if="store.searching && !store.result"
      class="flex items-center gap-2 py-10 text-muted-foreground"
      role="status"
    >
      <MdSpinner />
      {{ t('openFileHolders.searching') }}
    </div>

    <MdEmptyState
      v-else-if="!store.result"
      :icon-name="ICON_NAMES.fileSearch"
      :title="t('openFileHolders.idleTitle')"
      :description="t('openFileHolders.idleDescription')"
    >
      <Button size="lg" type="button" :disabled="store.busy" @click="store.find(store.path)">
        <MdIcon :name="ICON_NAMES.search" :size="17" />
        {{ t('openFileHolders.find') }}
      </Button>
    </MdEmptyState>

    <template v-else>
      <MdEmptyState
        v-if="!applications.length"
        :icon-name="ICON_NAMES.fileSearch"
        :title="t(store.result.path ? 'openFileHolders.emptyTitle' : 'openFileHolders.emptyAllTitle')"
        :description="t(store.result.path ? 'openFileHolders.emptyDescription' : 'openFileHolders.emptyAllDescription')"
      />
      <template v-else>
        <div class="flex flex-wrap items-center justify-between gap-2 pb-2" :aria-busy="store.searching">
          <span class="text-sm text-muted-foreground">
            {{
              t('openFileHolders.summary', {
                applications: FormatUtils.integer(totals.applications),
                processes: FormatUtils.integer(totals.processes),
                files: FormatUtils.integer(totals.files),
              })
            }}
          </span>
          <span v-if="store.result.path" class="min-w-0 truncate text-xs text-muted-foreground">
            {{ t('openFileHolders.holdingPath', { path: store.result.path }) }}
          </span>
        </div>
        <MdInlineNotice class="mb-3" tone="info">{{ t('openFileHolders.visibilityNotice') }}</MdInlineNotice>
        <ul class="m-0 flex list-none flex-col gap-1.5 p-0">
          <MdOpenFileApplicationRow
            v-for="application in applications"
            :key="application.key"
            :application="application"
            :expanded="expanded.has(application.key)"
            :busy="store.busy"
            @toggle="toggle(application.key)"
            @reveal="reveal"
            @close="mode => (pending = { application, mode })"
          />
        </ul>
      </template>
    </template>

    <MdConfirmDialog
      v-model:open="confirmOpen"
      :title="t(forcing ? 'openFileHolders.forceConfirmTitle' : 'openFileHolders.confirmTitle', { name: pendingName })"
      :description="t(forcing ? 'openFileHolders.forceConfirmDescription' : 'openFileHolders.confirmDescription')"
      :cancel-label="t('common.cancel')"
      :confirm-label="t(forcing ? 'openFileHolders.forceClose' : 'openFileHolders.close')"
      :confirm-variant="forcing ? 'destructive' : 'default'"
      @confirm="confirmClose"
    />
  </MdPageShell>
</template>
