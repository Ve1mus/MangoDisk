<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';

import MdCopyButton from '@/components/custom/md-copy-button.vue';
import MdStatusBadge from '@/components/custom/md-status-badge.vue';
import MdIcon from '@/components/icons/md-icon.vue';
import { Button } from '@/components/ui/button';
import type { HomebrewPackage, HomebrewPackageKind, HomebrewRole } from '@/lib/models/developer-environments';
import { ICON_NAMES } from '@/lib/models/ui';
import { ByteSizeService } from '@/lib/services/byte-size-service';
import { homebrewRole, homebrewUninstallCommand } from '@/lib/utils/developer-environments';
import * as FormatUtils from '@/lib/utils/format';

const props = defineProps<{
  item: HomebrewPackage;
  expanded: boolean;
  busy: boolean;
  removing: boolean;
  /** Names of installed formulae, so a dependency that is installed can be followed. */
  installedNames: ReadonlySet<string>;
  dateText: string;
}>();
const emit = defineEmits<{ toggle: []; uninstall: []; follow: [name: string] }>();
const { locale, t } = useI18n({ useScope: 'global' });

// Literal keys keep every message discoverable by the locale usage check.
const KIND_KEYS: Record<HomebrewPackageKind, string> = {
  formula: 'developerEnvironments.homebrew.kinds.formula',
  cask: 'developerEnvironments.homebrew.kinds.cask',
};
const ROLE_KEYS: Record<HomebrewRole, string> = {
  requested: 'developerEnvironments.homebrew.roles.requested',
  dependency: 'developerEnvironments.homebrew.roles.dependency',
  unusedDependency: 'developerEnvironments.homebrew.roles.unusedDependency',
};
const ROLE_TONES: Record<HomebrewRole, 'neutral' | 'primary' | 'warning'> = {
  requested: 'primary',
  dependency: 'neutral',
  unusedDependency: 'warning',
};

const role = computed(() => homebrewRole(props.item));
const blocked = computed(() => props.item.requiredBy.length > 0);
const detailsId = computed(() => `homebrew-details-${props.item.kind}-${props.item.name}`);
</script>

<template>
  <li :id="`homebrew-${item.kind}-${item.name}`" class="rounded-lg border border-border">
    <div class="flex items-center gap-3 px-3 py-2">
      <button
        type="button"
        class="flex min-w-0 flex-1 items-center gap-2 rounded-md text-left focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
        :aria-expanded="expanded"
        :aria-controls="detailsId"
        @click="emit('toggle')"
      >
        <MdIcon
          class="shrink-0 text-muted-foreground"
          :name="expanded ? ICON_NAMES.chevronUp : ICON_NAMES.chevronDown"
          :size="14"
        />
        <span class="min-w-0 flex-1">
          <span class="flex flex-wrap items-center gap-2">
            <span class="font-medium">{{ item.name }}</span>
            <MdStatusBadge size="compact">{{ t(KIND_KEYS[item.kind]) }}</MdStatusBadge>
            <MdStatusBadge size="compact" :tone="ROLE_TONES[role]">{{ t(ROLE_KEYS[role]) }}</MdStatusBadge>
          </span>
          <span class="m-0 mt-0.5 block truncate text-xs text-muted-foreground">
            {{ t('developerEnvironments.homebrew.versions', { versions: item.versions.join(', ') }) }}
            <template v-if="dateText">
              · {{ t('developerEnvironments.homebrew.installedAt', { date: dateText }) }}
            </template>
            <template v-if="item.dependencies.length">
              ·
              {{
                t('developerEnvironments.homebrew.dependsOnCount', {
                  count: FormatUtils.integer(item.dependencies.length),
                })
              }}
            </template>
            <template v-if="item.requiredBy.length">
              ·
              {{
                t('developerEnvironments.homebrew.requiredByCount', {
                  count: FormatUtils.integer(item.requiredBy.length),
                })
              }}
            </template>
          </span>
        </span>
      </button>
      <span class="shrink-0 text-sm tabular-nums">{{ ByteSizeService.bytes(item.bytes) }}</span>
      <MdCopyButton :text="homebrewUninstallCommand(item)" />
      <Button
        variant="outline"
        type="button"
        size="sm"
        class="hover:border-destructive/50 hover:text-destructive"
        :disabled="busy || blocked"
        :aria-busy="removing"
        @click="emit('uninstall')"
      >
        <MdIcon :name="ICON_NAMES.trash" :size="14" />
        {{ t('developerEnvironments.homebrew.uninstall') }}
      </Button>
    </div>

    <div v-if="expanded" :id="detailsId" class="grid gap-3 border-t border-border px-3 py-2.5 sm:grid-cols-2">
      <section v-for="group in ['dependencies', 'requiredBy'] as const" :key="group" class="min-w-0">
        <h3 class="m-0 mb-1.5 text-xs font-medium text-muted-foreground">
          {{
            t(
              group === 'dependencies'
                ? 'developerEnvironments.homebrew.dependsOn'
                : 'developerEnvironments.homebrew.requiredBy',
              { count: FormatUtils.integer(item[group].length) }
            )
          }}
        </h3>
        <p v-if="!item[group].length" class="m-0 text-xs text-muted-foreground">
          {{ t('developerEnvironments.homebrew.noRelations') }}
        </p>
        <ul v-else class="m-0 flex list-none flex-wrap gap-1.5 p-0">
          <li v-for="name in item[group]" :key="name">
            <button
              v-if="installedNames.has(name)"
              type="button"
              class="rounded-full border border-border px-2 py-0.5 text-xs hover:border-primary/60 hover:text-primary focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
              @click="emit('follow', name)"
            >
              {{ name }}
            </button>
            <span
              v-else
              class="rounded-full border border-dashed border-border px-2 py-0.5 text-xs text-muted-foreground"
            >
              {{ name }}
            </span>
          </li>
        </ul>
      </section>
      <p v-if="blocked" class="m-0 text-xs text-muted-foreground sm:col-span-2">
        {{ t('developerEnvironments.homebrew.blocked', { names: FormatUtils.list(item.requiredBy, locale) }) }}
      </p>
    </div>
  </li>
</template>
