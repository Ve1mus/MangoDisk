<script setup lang="ts">
import { computed, nextTick, ref } from 'vue';
import { useI18n } from 'vue-i18n';

import MdDestructiveActionDialog from '@/components/custom/md-destructive-action-dialog.vue';
import MdEmptyState from '@/components/custom/md-empty-state.vue';
import MdIconAction from '@/components/custom/md-icon-action.vue';
import MdInlineNotice from '@/components/custom/md-inline-notice.vue';
import MdMiddleEllipsis from '@/components/custom/md-middle-ellipsis.vue';
import MdPageShell from '@/components/custom/md-page-shell.vue';
import MdSpinner from '@/components/custom/md-spinner.vue';
import MdStatusBadge from '@/components/custom/md-status-badge.vue';
import MdIcon from '@/components/icons/md-icon.vue';
import { Button } from '@/components/ui/button';
import type {
  DeveloperEnvironmentNotice,
  HomebrewPackage,
  PythonEnvironment,
  PythonEnvironmentFlag,
} from '@/lib/models/developer-environments';
import { ICON_NAMES } from '@/lib/models/ui';
import { ByteSizeService } from '@/lib/services/byte-size-service';
import { FileManagerService } from '@/lib/services/file-manager-service';
import { pythonEnvironmentFlags, pythonEnvironmentName } from '@/lib/utils/developer-environments';
import * as FormatUtils from '@/lib/utils/format';
import { useAppStore } from '@/stores/app-store';
import { homebrewKey, useDeveloperEnvironmentsStore } from '@/stores/developer-environments-store';

import MdHomebrewPackageRow from './components/md-homebrew-package-row.vue';

type Tab = 'homebrew' | 'python';

const { locale, t } = useI18n({ useScope: 'global' });
const store = useDeveloperEnvironmentsStore();
const app = useAppStore();
const tab = ref<Tab>('homebrew');
const expanded = ref<ReadonlySet<string>>(new Set());
const pendingHomebrew = ref<HomebrewPackage | null>(null);
const pendingPython = ref<PythonEnvironment | null>(null);

// Literal keys keep every message discoverable by the locale usage check.
const TAB_KEYS: Record<Tab, string> = {
  homebrew: 'developerEnvironments.tabs.homebrew',
  python: 'developerEnvironments.tabs.python',
};
const FLAG_KEYS: Record<PythonEnvironmentFlag, string> = {
  interpreterMissing: 'developerEnvironments.python.flags.interpreterMissing',
  homeRoot: 'developerEnvironments.python.flags.homeRoot',
  noProject: 'developerEnvironments.python.flags.noProject',
  toolManaged: 'developerEnvironments.python.flags.toolManaged',
  systemSitePackages: 'developerEnvironments.python.flags.systemSitePackages',
};
const FLAG_TONES: Record<PythonEnvironmentFlag, 'neutral' | 'warning' | 'destructive'> = {
  interpreterMissing: 'destructive',
  homeRoot: 'warning',
  noProject: 'warning',
  toolManaged: 'neutral',
  systemSitePackages: 'neutral',
};
const HOMEBREW_NOTICE_KEYS = {
  removed: 'developerEnvironments.homebrew.notices.removed',
  notInstalled: 'developerEnvironments.homebrew.notices.notInstalled',
  stillRequired: 'developerEnvironments.homebrew.notices.stillRequired',
  stillInstalled: 'developerEnvironments.homebrew.notices.stillInstalled',
  failed: 'developerEnvironments.homebrew.notices.failed',
} as const;
const PYTHON_NOTICE_KEYS = {
  removed: 'developerEnvironments.python.notices.removed',
  notFound: 'developerEnvironments.python.notices.notFound',
  notAnEnvironment: 'developerEnvironments.python.notices.notAnEnvironment',
  outsideScope: 'developerEnvironments.python.notices.outsideScope',
  unexpectedContents: 'developerEnvironments.python.notices.unexpectedContents',
  failed: 'developerEnvironments.python.notices.failed',
} as const;

const homebrewPackages = computed(() => store.homebrew?.packages ?? []);
const pythonEnvironments = computed(() => store.python?.environments ?? []);
const installedNames = computed<ReadonlySet<string>>(
  () => new Set(homebrewPackages.value.filter(item => item.kind === 'formula').map(item => item.name))
);
const scanningCurrent = computed(() => (tab.value === 'homebrew' ? store.scanningHomebrew : store.scanningPython));
const scannedCurrent = computed(() => (tab.value === 'homebrew' ? store.homebrew : store.python) !== null);
const summary = computed(() => {
  const inventory = tab.value === 'homebrew' ? store.homebrew : store.python;
  if (!inventory) return '';
  const count = tab.value === 'homebrew' ? homebrewPackages.value.length : pythonEnvironments.value.length;
  return t('developerEnvironments.summary', {
    count: FormatUtils.integer(count),
    size: ByteSizeService.bytes(inventory.totalBytes),
  });
});
const notice = computed(() => {
  const current: DeveloperEnvironmentNotice | null = store.notice;
  if (!current) return null;
  const size = ByteSizeService.bytes(current.releasedBytes);
  if (current.kind === 'homebrew') {
    return {
      tone: current.outcome === 'removed' || current.outcome === 'notInstalled' ? 'success' : 'warning',
      text: t(HOMEBREW_NOTICE_KEYS[current.outcome], {
        name: current.name,
        size,
        names: FormatUtils.list(current.requiredBy, locale.value),
      }),
    } as const;
  }
  return {
    tone: current.outcome === 'removed' || current.outcome === 'notFound' ? 'success' : 'warning',
    text: t(PYTHON_NOTICE_KEYS[current.outcome], { name: pythonEnvironmentName({ path: current.name }), size }),
  } as const;
});

function scanCurrent() {
  return tab.value === 'homebrew' ? store.scanHomebrew() : store.scanPython();
}

function date(timestamp: number | null): string {
  return timestamp ? FormatUtils.dateTime(timestamp, locale.value) : '';
}

function toggle(key: string) {
  const next = new Set(expanded.value);
  if (!next.delete(key)) next.add(key);
  expanded.value = next;
}

/** Opens a related package and brings it into view, so a dependency chain can be walked. */
async function follow(name: string) {
  const key = homebrewKey({ kind: 'formula', name });
  expanded.value = new Set(expanded.value).add(key);
  await nextTick();
  document.getElementById(`homebrew-${key.replace(':', '-')}`)?.scrollIntoView({ block: 'nearest' });
}

async function confirmUninstall() {
  const item = pendingHomebrew.value;
  pendingHomebrew.value = null;
  if (item) await store.uninstallHomebrew(item);
}

async function confirmDelete() {
  const item = pendingPython.value;
  pendingPython.value = null;
  if (item) await store.deletePython(item);
}

function dialogOpen(pending: typeof pendingHomebrew | typeof pendingPython) {
  return computed({
    get: () => pending.value !== null,
    set: open => {
      if (!open) pending.value = null;
    },
  });
}
const uninstallDialogOpen = dialogOpen(pendingHomebrew);
const deleteDialogOpen = dialogOpen(pendingPython);

async function reveal(path: string) {
  try {
    await FileManagerService.reveal(path);
  } catch (error) {
    app.reportError(error);
  }
}
</script>

<template>
  <MdPageShell :title="t('developerEnvironments.title')" :subtitle="t('developerEnvironments.subtitle')">
    <template #actions>
      <Button v-if="scannedCurrent" variant="outline" type="button" :disabled="store.busy" @click="scanCurrent()">
        <MdIcon :name="ICON_NAMES.refresh" :size="16" />
        {{ t('developerEnvironments.rescan') }}
      </Button>
    </template>

    <div class="flex items-center justify-between gap-3 pb-3">
      <div class="flex gap-2" role="tablist">
        <Button
          v-for="id in ['homebrew', 'python'] as const"
          :key="id"
          role="tab"
          type="button"
          size="sm"
          :aria-selected="tab === id"
          :variant="tab === id ? 'default' : 'outline'"
          @click="tab = id"
        >
          {{ t(TAB_KEYS[id]) }}
        </Button>
      </div>
      <span class="text-sm text-muted-foreground">{{ summary }}</span>
    </div>

    <div
      v-if="store.removing !== null"
      class="mb-3 flex items-center gap-2 text-sm text-muted-foreground"
      role="status"
    >
      <MdSpinner />
      {{ t('developerEnvironments.removing') }}
    </div>
    <MdInlineNotice v-else-if="notice" class="mb-3" :tone="notice.tone" role="status">{{ notice.text }}</MdInlineNotice>

    <div v-if="scanningCurrent" class="flex items-center gap-2 py-10 text-muted-foreground" role="status">
      <MdSpinner />
      {{ t('developerEnvironments.scanning') }}
    </div>

    <template v-else-if="tab === 'homebrew'">
      <MdEmptyState
        v-if="!store.homebrew"
        :icon-name="ICON_NAMES.package"
        :title="t('developerEnvironments.homebrew.idleTitle')"
        :description="t('developerEnvironments.homebrew.idleDescription')"
      >
        <Button size="lg" type="button" :disabled="store.busy" @click="store.scanHomebrew()">
          <MdIcon :name="ICON_NAMES.search" :size="17" />
          {{ t('developerEnvironments.scan') }}
        </Button>
      </MdEmptyState>
      <MdEmptyState
        v-else-if="!store.homebrew.supported"
        :icon-name="ICON_NAMES.package"
        :title="t('developerEnvironments.homebrew.unsupportedTitle')"
        :description="t('developerEnvironments.homebrew.unsupportedDescription')"
      />
      <MdEmptyState
        v-else-if="!homebrewPackages.length"
        :icon-name="ICON_NAMES.package"
        :title="t('developerEnvironments.homebrew.emptyTitle')"
        :description="t('developerEnvironments.homebrew.emptyDescription')"
      />
      <template v-else>
        <MdInlineNotice class="mb-3" tone="info">{{ t('developerEnvironments.homebrew.hint') }}</MdInlineNotice>
        <ul class="m-0 flex list-none flex-col gap-1.5 p-0">
          <MdHomebrewPackageRow
            v-for="item in homebrewPackages"
            :key="homebrewKey(item)"
            :item="item"
            :expanded="expanded.has(homebrewKey(item))"
            :busy="store.busy"
            :removing="store.removing === homebrewKey(item)"
            :installed-names="installedNames"
            :date-text="date(item.installedAtMs)"
            @toggle="toggle(homebrewKey(item))"
            @uninstall="pendingHomebrew = item"
            @follow="follow"
          />
        </ul>
      </template>
    </template>

    <template v-else>
      <MdEmptyState
        v-if="!store.python"
        :icon-name="ICON_NAMES.code"
        :title="t('developerEnvironments.python.idleTitle')"
        :description="t('developerEnvironments.python.idleDescription')"
      >
        <Button size="lg" type="button" :disabled="store.busy" @click="store.scanPython()">
          <MdIcon :name="ICON_NAMES.search" :size="17" />
          {{ t('developerEnvironments.scan') }}
        </Button>
      </MdEmptyState>
      <MdEmptyState
        v-else-if="!pythonEnvironments.length"
        :icon-name="ICON_NAMES.code"
        :title="t('developerEnvironments.python.emptyTitle')"
        :description="t('developerEnvironments.python.emptyDescription')"
      />
      <template v-else>
        <MdInlineNotice class="mb-3" tone="info">{{ t('developerEnvironments.python.hint') }}</MdInlineNotice>
        <MdInlineNotice v-if="!store.python.complete" class="mb-3" tone="warning">
          {{ t('developerEnvironments.python.incomplete') }}
        </MdInlineNotice>
        <ul class="m-0 flex list-none flex-col gap-1.5 p-0">
          <li
            v-for="item in pythonEnvironments"
            :key="item.path"
            class="flex items-center gap-3 rounded-lg border border-border px-3 py-2"
          >
            <div class="min-w-0 flex-1">
              <div class="flex flex-wrap items-center gap-2">
                <span class="font-medium">{{ pythonEnvironmentName(item) }}</span>
                <MdStatusBadge
                  v-for="flag in pythonEnvironmentFlags(item)"
                  :key="flag"
                  size="compact"
                  :tone="FLAG_TONES[flag]"
                >
                  {{ t(FLAG_KEYS[flag]) }}
                </MdStatusBadge>
              </div>
              <p class="m-0 mt-0.5 flex min-w-0 items-center gap-1 text-xs text-muted-foreground">
                <MdMiddleEllipsis class="min-w-0" :text="item.path" :tail-length="24" />
              </p>
              <p class="m-0 mt-0.5 truncate text-xs text-muted-foreground">
                <template v-if="item.pythonVersion">
                  {{ t('developerEnvironments.python.version', { version: item.pythonVersion }) }} ·
                </template>
                {{ t('developerEnvironments.python.files', { count: FormatUtils.integer(item.fileCount) }) }}
                <template v-if="item.createdAtMs">
                  · {{ t('developerEnvironments.python.createdAt', { date: date(item.createdAtMs) }) }}
                </template>
              </p>
            </div>
            <span class="shrink-0 text-sm tabular-nums">{{ ByteSizeService.bytes(item.bytes) }}</span>
            <MdIconAction :label="t('developerEnvironments.python.reveal')" @click="reveal(item.path)">
              <MdIcon :name="ICON_NAMES.folderOpen" :size="16" />
            </MdIconAction>
            <Button
              variant="outline"
              type="button"
              size="sm"
              class="hover:border-destructive/50 hover:text-destructive"
              :disabled="store.busy"
              :aria-busy="store.removing === item.path"
              @click="pendingPython = item"
            >
              <MdIcon :name="ICON_NAMES.trash" :size="14" />
              {{ t('developerEnvironments.python.delete') }}
            </Button>
          </li>
        </ul>
      </template>
    </template>

    <MdDestructiveActionDialog
      v-model:open="uninstallDialogOpen"
      :title="t('developerEnvironments.homebrew.uninstallTitle', { name: pendingHomebrew?.name ?? '' })"
      :description="t('developerEnvironments.homebrew.uninstallDescription')"
      :summary-label="t('developerEnvironments.homebrew.uninstallSummary')"
      :summary-value="ByteSizeService.bytes(pendingHomebrew?.bytes ?? 0)"
      :note="
        t(
          pendingHomebrew?.kind === 'cask'
            ? 'developerEnvironments.homebrew.caskNote'
            : 'developerEnvironments.homebrew.formulaNote'
        )
      "
      :cancel-label="t('common.cancel')"
      :confirm-label="t('developerEnvironments.homebrew.uninstall')"
      @confirm="confirmUninstall"
    />
    <MdDestructiveActionDialog
      v-model:open="deleteDialogOpen"
      :title="
        t('developerEnvironments.python.deleteTitle', {
          name: pendingPython ? pythonEnvironmentName(pendingPython) : '',
        })
      "
      :description="t('developerEnvironments.python.deleteDescription', { path: pendingPython?.path ?? '' })"
      :summary-label="t('developerEnvironments.python.deleteSummary')"
      :summary-value="ByteSizeService.bytes(pendingPython?.bytes ?? 0)"
      :note="
        t(
          pendingPython?.toolManaged
            ? 'developerEnvironments.python.toolManagedNote'
            : 'developerEnvironments.python.deleteNote'
        )
      "
      :cancel-label="t('common.cancel')"
      :confirm-label="t('developerEnvironments.python.delete')"
      @confirm="confirmDelete"
    />
  </MdPageShell>
</template>
