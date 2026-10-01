import { defineStore } from 'pinia';

import type { ApplicationCloseMode } from '@/lib/models/application-close';
import type { OpenFileHolderCloseOutcome, OpenFileHoldersResult } from '@/lib/models/open-file-holders';
import { LOG_DOMAINS, LOG_EVENTS } from '@/lib/models/telemetry';
import { LoggerService } from '@/lib/services/logger-service';
import { OpenFileHolderService } from '@/lib/services/open-file-holder-service';
import type { OpenFileApplication } from '@/lib/utils/open-file-holders';

import { useAppStore } from './app-store';

interface OpenFileHoldersState {
  /** Optional filter; empty lists every application with open files. */
  path: string;
  result: OpenFileHoldersResult | null;
  searching: boolean;
  closingKey: string | null;
  closeOutcome: OpenFileHolderCloseOutcome | null;
}

export const useOpenFileHoldersStore = defineStore('openFileHolders', {
  state: (): OpenFileHoldersState => ({
    path: '',
    result: null,
    searching: false,
    closingKey: null,
    closeOutcome: null,
  }),
  getters: {
    busy: state => state.searching || state.closingKey !== null,
  },
  actions: {
    async find(path?: string) {
      if (this.searching) return;
      const target = (path ?? this.path).trim();
      this.path = target;
      this.searching = true;
      try {
        this.result = await OpenFileHolderService.find(target || null);
      } catch (error) {
        LoggerService.warn(LOG_DOMAINS.openFileHolders, LOG_EVENTS.openFileHoldersFailed, { path: target, error });
        useAppStore().reportError(error);
      } finally {
        this.searching = false;
      }
    },
    async close(application: OpenFileApplication, mode: ApplicationCloseMode) {
      if (!application.executablePaths.length || this.busy) return;
      const target = this.result?.path ?? null;
      this.closingKey = application.key;
      this.closeOutcome = null;
      try {
        const closed = await OpenFileHolderService.close(target, application.executablePaths, mode);
        this.closeOutcome =
          closed.failedTargetCount === 0 && closed.remainingProcessCount === 0 ? 'closed' : 'stillRunning';
      } catch (error) {
        LoggerService.warn(LOG_DOMAINS.openFileHolders, LOG_EVENTS.openFileHolderCloseFailed, {
          path: target,
          application: application.name,
          error,
        });
        useAppStore().reportError(error);
      } finally {
        this.closingKey = null;
      }
      // Always re-read: the live list, not the close result, shows what still holds files.
      await this.find(target ?? '');
    },
  },
});
