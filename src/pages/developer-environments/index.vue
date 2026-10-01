<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';

import MdCopyButton from '@/components/custom/md-copy-button.vue';
import MdEmptyState from '@/components/custom/md-empty-state.vue';
import MdIconAction from '@/components/custom/md-icon-action.vue';
import MdInlineNotice from '@/components/custom/md-inline-notice.vue';
import MdMiddleEllipsis from '@/components/custom/md-middle-ellipsis.vue';
import MdPageShell from '@/components/custom/md-page-shell.vue';
import MdSpinner from '@/components/custom/md-spinner.vue';
import MdStatusBadge from '@/components/custom/md-status-badge.vue';
import MdIcon from '@/components/icons/md-icon.vue';
import { Button } from '@/components/ui/button';
import type { HomebrewPackageKind, HomebrewRole, PythonEnvironmentFlag } from '@/lib/models/developer-environments';
import { ICON_NAMES } from '@/lib/models/ui';
import { ByteSizeService } from '@/lib/services/byte-size-service';
import { FileManagerService } from '@/lib/services/file-manager-service';
import {
  homebrewRole,
  homebrewUninstallCommand,
  pythonEnvironmentFlags,
  pythonEnvironmentName,
} from '@/lib/utils/developer-environments';
import * as FormatUtils from '@/lib/utils/format';
import { useAppStore } from '@/stores/app-store';
import { useDeveloperEnvironmentsStore } from '@/stores/developer-environments-store';

type Tab = 'homebrew' | 'python';

const { locale, t } = useI18n({ useScope: 'global' });
const store = useDeveloperEnvironmentsStore();
const app = useAppStore();
const tab = ref<Tab>('homebrew');

// Literal keys keep every message discoverable by the locale usage check.
const TAB_KEYS: Record<Tab, string> = {
  homebrew: 'developerEnvironments.tabs.homebrew',
  python: 'developerEnvironments.tabs.python',
};
const KIND_KEYS: Record<HomebrewPackageKind, string> = {
  formula: 'developerEnvironments.homebrew.kinds.formula',
  cask: 'developerEnvironments.homebrew.kinds.cask',
};
const ROLE_KEYS: Record<HomebrewRole, string> = {
  requested: 'developerEnvironments.homebrew.roles.requested',
  dependency: 'developerEnvironments.homebrew.roles.dependency',
  unusedDependency: 'developerEnvironments.homebrew.roles.unusedDependency',
};
const FLAG_KEYS: Record<PythonEnvironmentFlag, string> = {
  interpreterMissing: 'developerEnvironments.python.flags.interpreterMissing',
  homeRoot: 'developerEnvironments.python.flags.homeRoot',
  noProject: 'developerEnvironments.python.flags.noProject',
  toolManaged: 'developerEnvironments.python.flags.toolManaged',
  systemSitePackages: 'developerEnvironments.python.flags.systemSitePackages',
};
const ROLE_TONES: Record<HomebrewRole, 'neutral' | 'primary' | 'warning'> = {
  requested: 'primary',
  dependency: 'neutral',
  unusedDependency: 'warning',
};
const FLAG_TONES: Record<PythonEnvironmentFlag, 'neutral' | 'warning' | 'destructive'> = {
  interpreterMissing: 'destructive',
  homeRoot: 'warning',
  noProject: 'warning',
  toolManaged: 'neutral',
  systemSitePackages: 'neutral',
};

const homebrewPackages = computed(() => store.homebrew?.packages ?? []);
const pythonEnvironments = computed(() => store.python?.environments ?? []);
const scanningCurrent = computed(() => (tab.value === 'homebrew' ? store.scanningHomebrew : store.scanningPython));
const summary = computed(() => {
  const inventory = tab.value === 'homebrew' ? store.homebrew : store.python;
  if (!inventory) return '';
  const count = tab.value === 'homebrew' ? homebrewPackages.value.length : pythonEnvironments.value.length;
  return t('developerEnvironments.summary', {
    count: FormatUtils.integer(count),
    size: ByteSizeService.bytes(inventory.totalBytes),
  });
});

function date(timestamp: number | null): string {
  return FormatUtils.dateTime(timestamp, locale.value);
}

async function reveal(path: string) {
  try {
    await FileManagerService.reveal(path);
  } catch (error) {
    app.reportError(error);
  }
}

// The page is kept alive, so it scans once on first display and then on request.
onMounted(() => {
  if (!store.homebrew && !store.python) void store.scanAll();
});
</script>

<template>
  <MdPageShell :title="t('developerEnvironments.title')" :subtitle="t('developerEnvironments.subtitle')">
    <template #actions>
      <Button variant="outline" type="button" :disabled="store.scanning" @click="store.scanAll()">
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

    <div v-if="scanningCurrent" class="flex items-center gap-2 py-10 text-muted-foreground" role="status">
      <MdSpinner />
      {{ t('developerEnvironments.scanning') }}
    </div>

    <template v-else-if="tab === 'homebrew'">
      <MdEmptyState
        v-if="store.homebrew && !store.homebrew.supported"
        :icon-name="ICON_NAMES.package"
        :title="t('developerEnvironments.homebrew.unsupportedTitle')"
        :description="t('developerEnvironments.homebrew.unsupportedDescription')"
      />
      <MdEmptyState
        v-else-if="store.homebrew && !homebrewPackages.length"
        :icon-name="ICON_NAMES.package"
        :title="t('developerEnvironments.homebrew.emptyTitle')"
        :description="t('developerEnvironments.homebrew.emptyDescription')"
      />
      <template v-else-if="homebrewPackages.length">
        <MdInlineNotice class="mb-3" tone="info">{{ t('developerEnvironments.homebrew.hint') }}</MdInlineNotice>
        <ul class="m-0 flex list-none flex-col gap-1.5 p-0">
          <li
            v-for="item in homebrewPackages"
            :key="`${item.kind}:${item.name}`"
            class="flex items-center gap-3 rounded-lg border border-border px-3 py-2"
          >
            <div class="min-w-0 flex-1">
              <div class="flex flex-wrap items-center gap-2">
                <span class="font-medium">{{ item.name }}</span>
                <MdStatusBadge size="compact">{{ t(KIND_KEYS[item.kind]) }}</MdStatusBadge>
                <MdStatusBadge size="compact" :tone="ROLE_TONES[homebrewRole(item)]">
                  {{ t(ROLE_KEYS[homebrewRole(item)]) }}
                </MdStatusBadge>
              </div>
              <p class="m-0 mt-0.5 truncate text-xs text-muted-foreground">
                {{ t('developerEnvironments.homebrew.versions', { versions: item.versions.join(', ') }) }}
                <template v-if="item.installedAtMs">
                  · {{ t('developerEnvironments.homebrew.installedAt', { date: date(item.installedAtMs) }) }}
                </template>
                <template v-if="item.requiredBy.length">
                  ·
                  {{
                    t('developerEnvironments.homebrew.requiredBy', { names: FormatUtils.list(item.requiredBy, locale) })
                  }}
                </template>
              </p>
            </div>
            <span class="shrink-0 text-sm tabular-nums">{{ ByteSizeService.bytes(item.bytes) }}</span>
            <MdCopyButton :text="homebrewUninstallCommand(item)" />
          </li>
        </ul>
      </template>
    </template>

    <template v-else>
      <MdEmptyState
        v-if="store.python && !pythonEnvironments.length"
        :icon-name="ICON_NAMES.code"
        :title="t('developerEnvironments.python.emptyTitle')"
        :description="t('developerEnvironments.python.emptyDescription')"
      />
      <template v-else-if="pythonEnvironments.length">
        <MdInlineNotice class="mb-3" tone="info">{{ t('developerEnvironments.python.hint') }}</MdInlineNotice>
        <MdInlineNotice v-if="store.python && !store.python.complete" class="mb-3" tone="warning">
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
          </li>
        </ul>
      </template>
    </template>
  </MdPageShell>
</template>
