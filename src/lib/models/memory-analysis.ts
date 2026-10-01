export type MemoryPressure = 'normal' | 'warning' | 'critical';

/** What a consumer's size measures: footprint includes compressed and swapped memory. */
export type MemoryMetric = 'footprint' | 'resident';

export interface MemoryCategoryBytes {
  applicationBytes: number;
  wiredBytes: number;
  compressedBytes: number;
  cachedBytes: number;
}

export interface MemoryAnalysisOverview {
  totalBytes: number;
  usedBytes: number;
  usedPercent: number;
  freeBytes: number;
  swapUsedBytes: number;
  swapTotalBytes: number;
  /** Absent where the platform does not expose these counters. */
  categories: MemoryCategoryBytes | null;
  pressure: MemoryPressure | null;
}

export interface MemoryConsumerProcess {
  pid: number;
  name: string;
  bytes: number;
  approximate: boolean;
}

export interface MemoryConsumer {
  id: string;
  name: string;
  bytes: number;
  processCount: number;
  processes: MemoryConsumerProcess[];
  iconPath: string | null;
  isBundle: boolean;
  canQuit: boolean;
  approximate: boolean;
}

export interface MemoryAnalysis {
  schemaVersion: number;
  sampledAtMs: number;
  metric: MemoryMetric;
  overview: MemoryAnalysisOverview;
  consumers: MemoryConsumer[];
  omittedProcessCount: number;
  elapsedMs: number;
}

export type MemorySegmentId = 'application' | 'wired' | 'compressed' | 'cached' | 'free';

export interface MemorySegment {
  id: MemorySegmentId;
  bytes: number;
  /** Share of installed memory, scaled so the segments never exceed the whole bar. */
  percent: number;
}

export type MemoryFindingCode = 'pressureCritical' | 'pressureWarning' | 'swapInUse' | 'heavyCompression';

export interface MemoryFinding {
  code: MemoryFindingCode;
  /** The measurement the explanation quotes, when it has one. */
  bytes: number | null;
}
