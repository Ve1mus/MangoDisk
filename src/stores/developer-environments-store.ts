import { defineStore } from 'pinia';

import type {
  DeveloperEnvironmentNotice,
  HomebrewInventory,
  HomebrewPackage,
  PythonEnvironment,
  PythonEnvironmentScan,
} from '@/lib/models/developer-environments';
import { LOG_DOMAINS, LOG_EVENTS } from '@/lib/models/telemetry';
import { DeveloperEnvironmentService } from '@/lib/services/developer-environment-service';
import { LoggerService } from '@/lib/services/logger-service';

import { useAppStore } from './app-store';

interface DeveloperEnvironmentsState {
  homebrew: HomebrewInventory | null;
  python: PythonEnvironmentScan | null;
  scanningHomebrew: boolean;
  scanningPython: boolean;
  /** Identity of the item being removed, so its row and the page can show progress. */
  removing: string | null;
  notice: DeveloperEnvironmentNotice | null;
}

export function homebrewKey(item: Pick<HomebrewPackage, 'kind' | 'name'>): string {
  return `${item.kind}:${item.name}`;
}

export const useDeveloperEnvironmentsStore = defineStore('developerEnvironments', {
  state: (): DeveloperEnvironmentsState => ({
    homebrew: null,
    python: null,
    scanningHomebrew: false,
    scanningPython: false,
    removing: null,
    notice: null,
  }),
  getters: {
    scanning: state => state.scanningHomebrew || state.scanningPython,
    busy: state => state.scanningHomebrew || state.scanningPython || state.removing !== null,
  },
  actions: {
    async scanHomebrew() {
      if (this.scanningHomebrew) return;
      this.scanningHomebrew = true;
      try {
        this.homebrew = await DeveloperEnvironmentService.scanHomebrew();
      } catch (error) {
        LoggerService.warn(LOG_DOMAINS.developerEnvironments, LOG_EVENTS.homebrewScanFailed, { error });
        useAppStore().reportError(error);
      } finally {
        this.scanningHomebrew = false;
      }
    },
    async scanPython() {
      if (this.scanningPython) return;
      this.scanningPython = true;
      try {
        this.python = await DeveloperEnvironmentService.scanPythonEnvironments();
      } catch (error) {
        LoggerService.warn(LOG_DOMAINS.developerEnvironments, LOG_EVENTS.pythonScanFailed, { error });
        useAppStore().reportError(error);
      } finally {
        this.scanningPython = false;
      }
    },
    async uninstallHomebrew(item: HomebrewPackage) {
      if (this.busy) return;
      this.removing = homebrewKey(item);
      this.notice = null;
      try {
        const result = await DeveloperEnvironmentService.uninstallHomebrewPackage(item.name, item.kind);
        this.notice = {
          kind: 'homebrew',
          name: item.name,
          outcome: result.outcome,
          requiredBy: result.requiredBy,
          releasedBytes: result.releasedBytes,
        };
      } catch (error) {
        LoggerService.warn(LOG_DOMAINS.developerEnvironments, LOG_EVENTS.homebrewUninstallFailed, {
          name: item.name,
          error,
        });
        useAppStore().reportError(error);
      } finally {
        this.removing = null;
      }
      // Dependents and roles change when a package goes, so re-read the inventory.
      await this.scanHomebrew();
    },
    async deletePython(item: PythonEnvironment) {
      if (this.busy) return;
      this.removing = item.path;
      this.notice = null;
      try {
        const result = await DeveloperEnvironmentService.deletePythonEnvironment(item.path);
        this.notice = {
          kind: 'python',
          name: item.path,
          outcome: result.outcome,
          releasedBytes: result.releasedBytes,
        };
        // A rescan can take far longer than the removal, so the list is updated in place.
        if (result.outcome === 'removed' || result.outcome === 'notFound') this.dropPython(item.path);
      } catch (error) {
        LoggerService.warn(LOG_DOMAINS.developerEnvironments, LOG_EVENTS.pythonDeleteFailed, {
          path: item.path,
          error,
        });
        useAppStore().reportError(error);
      } finally {
        this.removing = null;
      }
    },
    dropPython(path: string) {
      if (!this.python) return;
      const environments = this.python.environments.filter(item => item.path !== path);
      this.python = {
        ...this.python,
        environments,
        totalBytes: environments.reduce((total, item) => total + item.bytes, 0),
      };
    },
  },
});
