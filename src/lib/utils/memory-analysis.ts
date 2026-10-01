import type {
  MemoryAnalysisOverview,
  MemoryFinding,
  MemorySegment,
  MemorySegmentId,
} from '@/lib/models/memory-analysis';

const GIBIBYTE = 1024 ** 3;
/** Swap in active use means macOS ran out of RAM for what is open. */
const SWAP_FINDING_BYTES = GIBIBYTE;
/** Compression this large means apps hold clearly more than fits in RAM. */
const COMPRESSION_FINDING_SHARE = 0.2;

/** Plain-language reasons the machine is under memory strain, most serious first. */
export function memoryFindings(overview: MemoryAnalysisOverview): MemoryFinding[] {
  const findings: MemoryFinding[] = [];
  if (overview.pressure === 'critical') findings.push({ code: 'pressureCritical', bytes: null });
  else if (overview.pressure === 'warning') findings.push({ code: 'pressureWarning', bytes: null });
  if (overview.swapUsedBytes >= SWAP_FINDING_BYTES) {
    findings.push({ code: 'swapInUse', bytes: overview.swapUsedBytes });
  }
  const compressed = overview.categories?.compressedBytes ?? 0;
  if (overview.totalBytes > 0 && compressed / overview.totalBytes >= COMPRESSION_FINDING_SHARE) {
    findings.push({ code: 'heavyCompression', bytes: compressed });
  }
  return findings;
}

/**
 * The stacked bar of where memory goes. The categories come from separate kernel
 * counters, so their sum can drift past the total by a few pages; scaling keeps the
 * bar honest instead of overflowing it.
 */
export function memorySegments(overview: MemoryAnalysisOverview): MemorySegment[] | null {
  const categories = overview.categories;
  if (!categories || overview.totalBytes <= 0) return null;
  const parts: Array<[MemorySegmentId, number]> = [
    ['application', categories.applicationBytes],
    ['wired', categories.wiredBytes],
    ['compressed', categories.compressedBytes],
    ['cached', categories.cachedBytes],
    ['free', overview.freeBytes],
  ];
  const sum = parts.reduce((total, [, bytes]) => total + bytes, 0);
  const scale = sum > overview.totalBytes ? overview.totalBytes / sum : 1;
  return parts.map(([id, bytes]) => ({
    id,
    bytes,
    percent: (bytes / overview.totalBytes) * 100 * scale,
  }));
}

/** Width of a row's bar against the largest consumer; footprints are a ranking, not shares of RAM. */
export function consumerShare(bytes: number, largestBytes: number): number {
  return largestBytes > 0 ? Math.min(100, (Math.max(0, bytes) / largestBytes) * 100) : 0;
}
