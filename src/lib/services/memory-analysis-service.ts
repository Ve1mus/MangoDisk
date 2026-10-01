import { invoke } from '@tauri-apps/api/core';

import type { MemoryAnalysis } from '@/lib/models/memory-analysis';

export class MemoryAnalysisService {
  static analyze(): Promise<MemoryAnalysis> {
    return invoke<MemoryAnalysis>('analyze_memory');
  }
}
