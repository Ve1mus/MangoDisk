import { defineStore } from 'pinia';

import type { HomebrewInventory, PythonEnvironmentScan } from '@/lib/models/developer-environments';
import { LOG_DOMAINS, LOG_EVENTS } from '@/lib/models/telemetry';
import { DeveloperEnvironmentService } from '@/lib/services/developer-environment-service';
import { LoggerService } from '@/lib/services/logger-service';

import { useAppStore } from './app-store';

interface DeveloperEnvironmentsState {
  homebrew: HomebrewInventory | null;
  python: PythonEnvironmentScan | null;
  scanningHomebrew: boolean;
  scanningPython: boolean;
}

export const useDeveloperEnvironmentsStore = defineStore('developerEnvironments', {
  state: (): DeveloperEnvironmentsState => ({
    homebrew: null,
    python: null,
    scanningHomebrew: false,
    scanningPython: false,
  }),
  getters: {
    scanning: state => state.scanningHomebrew || state.scanningPython,
  },
  actions: {
    async scanAll() {
      await Promise.all([this.scanHomebrew(), this.scanPython()]);
    },
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
  },
});
