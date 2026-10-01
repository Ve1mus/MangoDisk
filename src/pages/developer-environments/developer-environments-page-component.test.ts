// @vitest-environment happy-dom
import { flushPromises, mount } from '@vue/test-utils';
import { createPinia } from 'pinia';
import { createI18n } from 'vue-i18n';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import Page from './index.vue';
import type { HomebrewInventory, PythonEnvironmentScan } from '@/lib/models/developer-environments';
import { DeveloperEnvironmentService } from '@/lib/services/developer-environment-service';
import en from '@/locales/en-US.json';
import ru from '@/locales/ru-RU.json';

vi.mock('@/lib/services/developer-environment-service', () => ({
  DeveloperEnvironmentService: { scanHomebrew: vi.fn(), scanPythonEnvironments: vi.fn() },
}));
vi.mock('@/lib/services/file-manager-service', () => ({ FileManagerService: { reveal: vi.fn() } }));
vi.mock('@/lib/services/operating-system-service', () => ({
  OperatingSystemService: { isMacOs: () => true, isWindows: () => false, isLinux: () => false },
}));
vi.mock('@/lib/services/logger-service', () => ({ LoggerService: { warn: vi.fn() } }));

const homebrew: HomebrewInventory = {
  schemaVersion: 1,
  supported: true,
  prefix: '/opt/homebrew',
  totalBytes: 3000,
  packages: [
    {
      name: 'glib',
      kind: 'formula',
      versions: ['2.86.0'],
      bytes: 2000,
      installedAtMs: null,
      installedOnRequest: false,
      requiredBy: [],
      path: '/opt/homebrew/Cellar/glib',
    },
    {
      name: 'claude-code',
      kind: 'cask',
      versions: ['2.1.274'],
      bytes: 1000,
      installedAtMs: null,
      installedOnRequest: true,
      requiredBy: [],
      path: '/opt/homebrew/Caskroom/claude-code',
    },
  ],
};
const python: PythonEnvironmentScan = {
  schemaVersion: 1,
  complete: false,
  elapsedMs: 5,
  totalBytes: 100,
  environments: [
    {
      path: '/Users/me/.venv',
      pythonVersion: '3.13.0',
      interpreterMissing: false,
      includeSystemSitePackages: false,
      hasProjectMarkers: false,
      inHomeRoot: true,
      toolManaged: false,
      bytes: 100,
      fileCount: 7,
      createdAtMs: null,
    },
  ],
};

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(DeveloperEnvironmentService.scanHomebrew).mockResolvedValue(homebrew);
  vi.mocked(DeveloperEnvironmentService.scanPythonEnvironments).mockResolvedValue(python);
});

function render(messages: typeof en = en) {
  return mount(Page, {
    global: {
      plugins: [createPinia(), createI18n({ legacy: false, locale: 'test', messages: { test: messages } })],
      stubs: { MdIcon: true, MdCopyButton: true, MdIconAction: true, MdMiddleEllipsis: true },
    },
  });
}

describe('developer environments page', () => {
  it('scans on first display and labels unused dependencies', async () => {
    const wrapper = render();
    await flushPromises();

    expect(DeveloperEnvironmentService.scanHomebrew).toHaveBeenCalledTimes(1);
    expect(DeveloperEnvironmentService.scanPythonEnvironments).toHaveBeenCalledTimes(1);
    const text = wrapper.text();
    expect(text).toContain('glib');
    expect(text).toContain('Unused dependency');
    expect(text).toContain('Installed by you');
    expect(text).toContain('Cask');
  });

  it('shows Python environments with their review flags and the incomplete-search notice', async () => {
    const wrapper = render();
    await flushPromises();
    await wrapper.findAll('[role="tab"]')[1].trigger('click');

    const text = wrapper.text();
    expect(text).toContain('In home folder');
    expect(text).toContain('No project nearby');
    expect(text).toContain('Python 3.13.0');
    expect(text).toContain('The search stopped early');
  });

  it('reports an absent Homebrew installation instead of an empty list', async () => {
    vi.mocked(DeveloperEnvironmentService.scanHomebrew).mockResolvedValue({
      ...homebrew,
      supported: false,
      packages: [],
      totalBytes: 0,
    });
    const wrapper = render();
    await flushPromises();

    expect(wrapper.text()).toContain('Homebrew is not installed');
  });

  it('renders Russian copy', async () => {
    const wrapper = render(ru as typeof en);
    await flushPromises();

    expect(wrapper.text()).toContain('Неиспользуемая зависимость');
  });
});
