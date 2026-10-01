import { defineStore } from 'pinia';

import type { MemoryAnalysis } from '@/lib/models/memory-analysis';
import { LOG_DOMAINS, LOG_EVENTS } from '@/lib/models/telemetry';
import { LoggerService } from '@/lib/services/logger-service';
import { MemoryAnalysisService } from '@/lib/services/memory-analysis-service';

import { useAppStore } from './app-store';

interface MemoryAnalysisState {
  analysis: MemoryAnalysis | null;
  analyzing: boolean;
}

export const useMemoryAnalysisStore = defineStore('memoryAnalysis', {
  state: (): MemoryAnalysisState => ({ analysis: null, analyzing: false }),
  actions: {
    async analyze() {
      if (this.analyzing) return;
      this.analyzing = true;
      try {
        this.analysis = await MemoryAnalysisService.analyze();
      } catch (error) {
        LoggerService.warn(LOG_DOMAINS.memoryAnalysis, LOG_EVENTS.memoryAnalysisFailed, { error });
        useAppStore().reportError(error);
      } finally {
        this.analyzing = false;
      }
    },
  },
});
