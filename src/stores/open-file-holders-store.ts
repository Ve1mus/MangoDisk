import { defineStore } from 'pinia';

import type { ApplicationCloseMode } from '@/lib/models/application-close';
import type { OpenFileHolder, OpenFileHolderCloseOutcome, OpenFileHoldersResult } from '@/lib/models/open-file-holders';
import { LOG_DOMAINS, LOG_EVENTS } from '@/lib/models/telemetry';
import { LoggerService } from '@/lib/services/logger-service';
import { OpenFileHolderService } from '@/lib/services/open-file-holder-service';

import { useAppStore } from './app-store';

interface OpenFileHoldersState {
  path: string;
  result: OpenFileHoldersResult | null;
  searching: boolean;
  closingPid: number | null;
  closeOutcome: OpenFileHolderCloseOutcome | null;
}

export const useOpenFileHoldersStore = defineStore('openFileHolders', {
  state: (): OpenFileHoldersState => ({
    path: '',
    result: null,
    searching: false,
    closingPid: null,
    closeOutcome: null,
  }),
  getters: {
    busy: state => state.searching || state.closingPid !== null,
  },
  actions: {
    async find(path?: string) {
      const target = (path ?? this.path).trim();
      if (!target || this.searching) return;
      this.path = target;
      this.searching = true;
      try {
        this.result = await OpenFileHolderService.find(target);
      } catch (error) {
        LoggerService.warn(LOG_DOMAINS.openFileHolders, LOG_EVENTS.openFileHoldersFailed, { path: target, error });
        useAppStore().reportError(error);
      } finally {
        this.searching = false;
      }
    },
    async close(holder: OpenFileHolder, mode: ApplicationCloseMode) {
      const target = this.result?.path;
      if (!target || !holder.executablePath || this.busy) return;
      this.closingPid = holder.pid;
      this.closeOutcome = null;
      try {
        const closed = await OpenFileHolderService.close(target, holder.executablePath, mode);
        this.closeOutcome =
          closed.failedTargetCount === 0 && closed.remainingProcessCount === 0 ? 'closed' : 'stillRunning';
      } catch (error) {
        LoggerService.warn(LOG_DOMAINS.openFileHolders, LOG_EVENTS.openFileHolderCloseFailed, {
          path: target,
          pid: holder.pid,
          error,
        });
        useAppStore().reportError(error);
      } finally {
        this.closingPid = null;
      }
      // Always re-read: the live list, not the close result, shows what still holds the path.
      await this.find(target);
    },
  },
});
