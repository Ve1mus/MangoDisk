<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';

import MdConfirmDialog from '@/components/custom/md-confirm-dialog.vue';
import MdEmptyState from '@/components/custom/md-empty-state.vue';
import MdIconAction from '@/components/custom/md-icon-action.vue';
import MdInlineNotice from '@/components/custom/md-inline-notice.vue';
import MdPageShell from '@/components/custom/md-page-shell.vue';
import MdSpinner from '@/components/custom/md-spinner.vue';
import MdStatusBadge from '@/components/custom/md-status-badge.vue';
import MdIcon from '@/components/icons/md-icon.vue';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import type { ApplicationCloseMode } from '@/lib/models/application-close';
import type { OpenFileHolder } from '@/lib/models/open-file-holders';
import { ICON_NAMES } from '@/lib/models/ui';
import { FileManagerService } from '@/lib/services/file-manager-service';
import { FolderSelectionService } from '@/lib/services/folder-selection-service';
import { holderDisplayName, holderIsHelper } from '@/lib/utils/open-file-holders';
import * as FormatUtils from '@/lib/utils/format';
import { useAppStore } from '@/stores/app-store';
import { useOpenFileHoldersStore } from '@/stores/open-file-holders-store';

const { t } = useI18n({ useScope: 'global' });
const store = useOpenFileHoldersStore();
const app = useAppStore();

const pending = ref<{ holder: OpenFileHolder; mode: ApplicationCloseMode } | null>(null);
const confirmOpen = computed({
  get: () => pending.value !== null,
  set: open => {
    if (!open) pending.value = null;
  },
});
const pendingName = computed(() => (pending.value ? holderDisplayName(pending.value.holder) : ''));
const forcing = computed(() => pending.value?.mode === 'force');

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
  if (request) await store.close(request.holder, request.mode);
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
      <Button type="submit" :disabled="store.busy || !store.path.trim()">
        <MdIcon :name="ICON_NAMES.search" :size="16" />
        {{ t('openFileHolders.find') }}
      </Button>
    </form>

    <MdInlineNotice v-if="store.closeOutcome === 'closed'" class="mb-3" tone="success" role="status">
      {{ t('openFileHolders.closed') }}
    </MdInlineNotice>
    <MdInlineNotice v-else-if="store.closeOutcome === 'stillRunning'" class="mb-3" tone="warning" role="status">
      {{ t('openFileHolders.stillRunning') }}
    </MdInlineNotice>

    <div v-if="store.searching" class="flex items-center gap-2 py-10 text-muted-foreground" role="status">
      <MdSpinner />
      {{ t('openFileHolders.searching') }}
    </div>

    <template v-else-if="store.result">
      <MdEmptyState
        v-if="!store.result.holders.length"
        :icon-name="ICON_NAMES.fileSearch"
        :title="t('openFileHolders.emptyTitle')"
        :description="t('openFileHolders.emptyDescription')"
      />
      <template v-else>
        <MdInlineNotice class="mb-3" tone="info">{{ t('openFileHolders.visibilityNotice') }}</MdInlineNotice>
        <ul class="m-0 flex list-none flex-col gap-1.5 p-0">
          <li
            v-for="holder in store.result.holders"
            :key="holder.pid"
            class="flex items-center gap-3 rounded-lg border border-border px-3 py-2"
          >
            <div class="min-w-0 flex-1">
              <div class="flex flex-wrap items-center gap-2">
                <span class="font-medium">{{ holderDisplayName(holder) }}</span>
                <MdStatusBadge v-if="holderIsHelper(holder)" size="compact">
                  {{ t('openFileHolders.helper', { command: holder.command }) }}
                </MdStatusBadge>
                <MdStatusBadge size="compact">{{ t('openFileHolders.pid', { pid: holder.pid }) }}</MdStatusBadge>
                <MdStatusBadge size="compact">
                  {{ t('openFileHolders.openFiles', { count: FormatUtils.integer(holder.openFileCount) }) }}
                </MdStatusBadge>
              </div>
              <p
                v-for="sample in holder.samplePaths"
                :key="sample"
                class="m-0 mt-0.5 truncate text-xs text-muted-foreground"
              >
                {{ sample }}
              </p>
            </div>
            <MdIconAction
              v-if="holder.executablePath"
              :label="t('common.showInFileManager')"
              @click="reveal(holder.executablePath)"
            >
              <MdIcon :name="ICON_NAMES.folderOpen" :size="16" />
            </MdIconAction>
            <Button
              variant="outline"
              type="button"
              size="sm"
              :disabled="store.busy || !holder.executablePath"
              @click="pending = { holder, mode: 'graceful' }"
            >
              {{ t('openFileHolders.close') }}
            </Button>
            <Button
              variant="outline"
              type="button"
              size="sm"
              :disabled="store.busy || !holder.executablePath"
              @click="pending = { holder, mode: 'force' }"
            >
              {{ t('openFileHolders.forceClose') }}
            </Button>
          </li>
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
