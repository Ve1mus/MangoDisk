// @vitest-environment happy-dom
import { flushPromises, mount } from '@vue/test-utils';
import { createPinia } from 'pinia';
import { createI18n } from 'vue-i18n';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import Page from './index.vue';
import type { MemoryAnalysis } from '@/lib/models/memory-analysis';
import { MemoryAnalysisService } from '@/lib/services/memory-analysis-service';
import { ResidentService } from '@/lib/services/resident-service';
import en from '@/locales/en-US.json';
import ru from '@/locales/ru-RU.json';

vi.mock('@/lib/services/memory-analysis-service', () => ({ MemoryAnalysisService: { analyze: vi.fn() } }));
vi.mock('@/lib/services/resident-service', () => ({ ResidentService: { quitApplication: vi.fn() } }));
vi.mock('@/lib/services/file-manager-service', () => ({ FileManagerService: { reveal: vi.fn() } }));
vi.mock('@/lib/services/logger-service', () => ({ LoggerService: { warn: vi.fn() } }));
vi.mock('@/lib/services/operating-system-service', () => ({
  OperatingSystemService: { isMacOs: () => true, isWindows: () => false, isLinux: () => false },
}));

const GIB = 1024 ** 3;
const analysis: MemoryAnalysis = {
  schemaVersion: 1,
  sampledAtMs: 1,
  metric: 'footprint',
  omittedProcessCount: 0,
  elapsedMs: 14,
  overview: {
    totalBytes: 16 * GIB,
    usedBytes: 13 * GIB,
    usedPercent: 81,
    freeBytes: GIB / 2,
    swapUsedBytes: 8 * GIB,
    swapTotalBytes: 9 * GIB,
    categories: { applicationBytes: 4 * GIB, wiredBytes: 4 * GIB, compressedBytes: 5 * GIB, cachedBytes: 2 * GIB },
    pressure: 'warning',
  },
  consumers: [
    {
      id: 'chrome',
      name: 'Google Chrome',
      bytes: 8 * GIB,
      processCount: 29,
      processes: [
        { pid: 11, name: 'Google Chrome Helper (Renderer)', bytes: 2 * GIB, approximate: false },
        { pid: 10, name: 'Google Chrome', bytes: GIB, approximate: true },
      ],
      iconPath: '/Applications/Google Chrome.app',
      isBundle: true,
      canQuit: true,
      approximate: true,
    },
    {
      id: 'node',
      name: 'node',
      bytes: GIB / 2,
      processCount: 1,
      processes: [{ pid: 7, name: 'node', bytes: GIB / 2, approximate: false }],
      iconPath: '/usr/local/bin/node',
      isBundle: false,
      canQuit: false,
      approximate: false,
    },
  ],
};

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(MemoryAnalysisService.analyze).mockResolvedValue(analysis);
});

function render(messages: typeof en = en) {
  return mount(Page, {
    global: {
      plugins: [createPinia(), createI18n({ legacy: false, locale: 'en-US', messages: { 'en-US': messages } })],
      stubs: { MdIcon: true, MdIconAction: true, MdNativeFileIcon: true },
    },
  });
}

async function analyze(wrapper: ReturnType<typeof render>) {
  await wrapper
    .findAll('button')
    .find(button => button.text() === 'Analyze memory')!
    .trigger('click');
  await flushPromises();
}

describe('memory analysis page', () => {
  it('measures nothing until Analyze is pressed', async () => {
    const wrapper = render();
    await flushPromises();

    expect(MemoryAnalysisService.analyze).not.toHaveBeenCalled();
    expect(wrapper.text()).toContain('Find what is using your memory');

    await analyze(wrapper);

    expect(MemoryAnalysisService.analyze).toHaveBeenCalledTimes(1);
  });

  it('shows how memory is divided and explains the strain', async () => {
    const wrapper = render();
    await analyze(wrapper);

    const text = wrapper.text();
    expect(text).toContain('81%');
    expect(text).toContain('App memory');
    expect(text).toContain('Compressed');
    expect(text).toContain('Memory pressure');
    expect(text).toContain('Elevated');
    expect(text).toContain('moved to disk (swap)');
    expect(text).toContain('is compressed');
  });

  it('ranks applications and marks approximate footprints', async () => {
    const wrapper = render();
    await analyze(wrapper);

    const text = wrapper.text();
    expect(text.indexOf('Google Chrome')).toBeLessThan(text.indexOf('node'));
    expect(text).toContain('29 processes');
    expect(text).toContain('Memory footprint');
    expect(text).toContain('≈');
  });

  it('lists the processes of an application when it is expanded', async () => {
    const wrapper = render();
    await analyze(wrapper);
    expect(wrapper.text()).not.toContain('Renderer');

    await wrapper.findAll('[aria-expanded]')[0].trigger('click');

    const text = wrapper.text();
    expect(text).toContain('Google Chrome Helper (Renderer)');
    expect(text).toContain('PID 11');
    expect(text).toContain('and 27 smaller processes');
  });

  it('offers to quit only applications that can be quit, through the backend identity', async () => {
    vi.mocked(ResidentService.quitApplication).mockResolvedValue('requested');
    const wrapper = render();
    await analyze(wrapper);
    await wrapper.findAll('[aria-expanded]')[1].trigger('click');
    expect(wrapper.text()).not.toContain('Quit application');

    await wrapper.findAll('[aria-expanded]')[0].trigger('click');
    await wrapper
      .findAll('button')
      .find(button => button.text() === 'Quit application')!
      .trigger('click');
    await flushPromises();

    expect(ResidentService.quitApplication).toHaveBeenCalledWith('chrome');
    expect(wrapper.text()).toContain('Quit request sent');
  });

  it('omits the category breakdown where the platform does not provide it', async () => {
    vi.mocked(MemoryAnalysisService.analyze).mockResolvedValue({
      ...analysis,
      metric: 'resident',
      overview: { ...analysis.overview, categories: null, pressure: null, swapUsedBytes: 0 },
    });
    const wrapper = render();
    await analyze(wrapper);

    expect(wrapper.text()).not.toContain('App memory');
    expect(wrapper.text()).toContain('Resident memory');
    expect(wrapper.text()).not.toContain('Footprint includes memory');
  });

  it('renders Russian copy', async () => {
    const wrapper = render(ru as typeof en);
    await wrapper
      .findAll('button')
      .find(button => button.text() === 'Проанализировать память')!
      .trigger('click');
    await flushPromises();

    expect(wrapper.text()).toContain('Память приложений');
  });
});
