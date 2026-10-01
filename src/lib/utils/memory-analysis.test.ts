import { describe, expect, it } from 'vitest';

import type { MemoryAnalysisOverview } from '@/lib/models/memory-analysis';

import { consumerShare, memoryFindings, memorySegments } from './memory-analysis';

const GIB = 1024 ** 3;
const overview = (overrides: Partial<MemoryAnalysisOverview> = {}): MemoryAnalysisOverview => ({
  totalBytes: 16 * GIB,
  usedBytes: 12 * GIB,
  usedPercent: 75,
  freeBytes: 1 * GIB,
  swapUsedBytes: 0,
  swapTotalBytes: 4 * GIB,
  categories: { applicationBytes: 6 * GIB, wiredBytes: 4 * GIB, compressedBytes: 2 * GIB, cachedBytes: 2 * GIB },
  pressure: 'normal',
  ...overrides,
});

describe('memory findings', () => {
  it('reports nothing for a machine that is not under strain', () => {
    expect(memoryFindings(overview())).toEqual([]);
  });

  it('puts the pressure level first and quotes swap and compression sizes', () => {
    const findings = memoryFindings(
      overview({
        pressure: 'critical',
        swapUsedBytes: 2 * GIB,
        categories: { applicationBytes: GIB, wiredBytes: GIB, compressedBytes: 5 * GIB, cachedBytes: 0 },
      })
    );

    expect(findings.map(finding => finding.code)).toEqual(['pressureCritical', 'swapInUse', 'heavyCompression']);
    expect(findings[1].bytes).toBe(2 * GIB);
    expect(findings[2].bytes).toBe(5 * GIB);
  });

  it('distinguishes elevated from critical pressure and ignores light swap use', () => {
    expect(memoryFindings(overview({ pressure: 'warning', swapUsedBytes: GIB / 2 })).map(item => item.code)).toEqual([
      'pressureWarning',
    ]);
  });

  it('works without category counters or pressure, as on platforms that do not expose them', () => {
    expect(memoryFindings(overview({ categories: null, pressure: null, swapUsedBytes: 3 * GIB }))).toEqual([
      { code: 'swapInUse', bytes: 3 * GIB },
    ]);
  });
});

describe('memory segments', () => {
  it('sizes each category against installed memory', () => {
    const segments = memorySegments(overview())!;

    expect(segments.map(segment => segment.id)).toEqual(['application', 'wired', 'compressed', 'cached', 'free']);
    expect(segments[0].percent).toBeCloseTo(37.5);
    expect(segments[3].percent).toBeCloseTo(12.5);
  });

  it('scales counters that drift past the total so the bar never overflows', () => {
    const segments = memorySegments(
      overview({
        freeBytes: 8 * GIB,
        categories: { applicationBytes: 8 * GIB, wiredBytes: 8 * GIB, compressedBytes: 8 * GIB, cachedBytes: 8 * GIB },
      })
    )!;

    expect(segments.reduce((total, segment) => total + segment.percent, 0)).toBeCloseTo(100);
  });

  it('has no segments without category counters', () => {
    expect(memorySegments(overview({ categories: null }))).toBeNull();
  });
});

describe('consumer share', () => {
  it('compares against the largest consumer and tolerates an empty ranking', () => {
    expect(consumerShare(50, 200)).toBe(25);
    expect(consumerShare(300, 200)).toBe(100);
    expect(consumerShare(10, 0)).toBe(0);
    expect(consumerShare(-5, 100)).toBe(0);
  });
});
