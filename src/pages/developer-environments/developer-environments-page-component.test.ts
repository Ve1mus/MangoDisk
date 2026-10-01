// @vitest-environment happy-dom
import { flushPromises, mount } from '@vue/test-utils';
import { createPinia } from 'pinia';
import { createI18n } from 'vue-i18n';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import Page from './index.vue';
import type {
  HomebrewInventory,
  HomebrewUninstallResult,
  PythonEnvironmentDeleteResult,
  PythonEnvironmentScan,
} from '@/lib/models/developer-environments';
import { DeveloperEnvironmentService } from '@/lib/services/developer-environment-service';
import en from '@/locales/en-US.json';
import ru from '@/locales/ru-RU.json';

vi.mock('@/lib/services/developer-environment-service', () => ({
  DeveloperEnvironmentService: {
    scanHomebrew: vi.fn(),
    scanPythonEnvironments: vi.fn(),
    uninstallHomebrewPackage: vi.fn(),
    deletePythonEnvironment: vi.fn(),
  },
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
  totalBytes: 6000,
  packages: [
    {
      name: 'glib',
      kind: 'formula',
      versions: ['2.86.0'],
      bytes: 2000,
      installedAtMs: null,
      installedOnRequest: false,
      requiredBy: [],
      dependencies: ['pcre2'],
      path: '/opt/homebrew/Cellar/glib',
    },
    {
      name: 'pcre2',
      kind: 'formula',
      versions: ['10.45'],
      bytes: 3000,
      installedAtMs: null,
      installedOnRequest: false,
      requiredBy: ['glib'],
      dependencies: [],
      path: '/opt/homebrew/Cellar/pcre2',
    },
    {
      name: 'claude-code',
      kind: 'cask',
      versions: ['2.1.274'],
      bytes: 1000,
      installedAtMs: null,
      installedOnRequest: true,
      requiredBy: [],
      dependencies: [],
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
const uninstalled: HomebrewUninstallResult = {
  schemaVersion: 1,
  name: 'glib',
  kind: 'formula',
  outcome: 'removed',
  releasedBytes: 2000,
  requiredBy: [],
  exitCode: null,
};
const deleted: PythonEnvironmentDeleteResult = {
  schemaVersion: 1,
  path: '/Users/me/.venv',
  outcome: 'removed',
  releasedBytes: 100,
  removedFileCount: 7,
};

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(DeveloperEnvironmentService.scanHomebrew).mockResolvedValue(homebrew);
  vi.mocked(DeveloperEnvironmentService.scanPythonEnvironments).mockResolvedValue(python);
  vi.mocked(DeveloperEnvironmentService.uninstallHomebrewPackage).mockResolvedValue(uninstalled);
  vi.mocked(DeveloperEnvironmentService.deletePythonEnvironment).mockResolvedValue(deleted);
});

function render(messages: typeof en = en) {
  return mount(Page, {
    global: {
      plugins: [createPinia(), createI18n({ legacy: false, locale: 'en-US', messages: { 'en-US': messages } })],
      stubs: {
        MdIcon: true,
        MdCopyButton: true,
        MdIconAction: true,
        MdMiddleEllipsis: true,
        MdDestructiveActionDialog: true,
      },
    },
  });
}

const buttonByText = (wrapper: ReturnType<typeof render>, text: string) =>
  wrapper.findAll('button').find(button => button.text() === text);

async function scanHomebrew(wrapper: ReturnType<typeof render>) {
  await buttonByText(wrapper, 'Scan')!.trigger('click');
  await flushPromises();
}

describe('developer environments page', () => {
  it('waits for the Scan button instead of scanning on first display', async () => {
    const wrapper = render();
    await flushPromises();

    expect(DeveloperEnvironmentService.scanHomebrew).not.toHaveBeenCalled();
    expect(DeveloperEnvironmentService.scanPythonEnvironments).not.toHaveBeenCalled();
    expect(wrapper.text()).toContain('Review Homebrew packages');

    await scanHomebrew(wrapper);

    expect(DeveloperEnvironmentService.scanHomebrew).toHaveBeenCalledTimes(1);
    expect(DeveloperEnvironmentService.scanPythonEnvironments).not.toHaveBeenCalled();
    const text = wrapper.text();
    expect(text).toContain('glib');
    expect(text).toContain('Unused dependency');
    expect(text).toContain('Installed by you');
    expect(text).toContain('Cask');
  });

  it('scans Python environments only when their tab asks for it', async () => {
    const wrapper = render();
    await wrapper.findAll('[role="tab"]')[1].trigger('click');
    expect(wrapper.text()).toContain('Find Python environments');

    await scanHomebrew(wrapper);

    expect(DeveloperEnvironmentService.scanHomebrew).not.toHaveBeenCalled();
    const text = wrapper.text();
    expect(text).toContain('In home folder');
    expect(text).toContain('No project nearby');
    expect(text).toContain('Python 3.13.0');
    expect(text).toContain('The search stopped early');
  });

  it('shows what a package depends on and what needs it', async () => {
    const wrapper = render();
    await scanHomebrew(wrapper);
    expect(wrapper.text()).not.toContain('Depends on (1)');

    await wrapper.findAll('[aria-expanded]')[0].trigger('click');

    expect(wrapper.text()).toContain('Depends on (1)');
    expect(wrapper.text()).toContain('Required by (0)');
    const followLink = wrapper.findAll('button').find(button => button.text() === 'pcre2');
    expect(followLink).toBeDefined();

    await followLink!.trigger('click');

    expect(wrapper.text()).toContain('Required by (1)');
  });

  it('cannot uninstall a package other formulae still need', async () => {
    const wrapper = render();
    await scanHomebrew(wrapper);

    const uninstall = wrapper.findAll('button').filter(button => button.text() === 'Uninstall');

    // Listed as scanned: glib (free to remove), pcre2 (needed by glib), then the cask.
    expect(uninstall[0].attributes('disabled')).toBeUndefined();
    expect(uninstall[1].attributes('disabled')).toBeDefined();
    expect(uninstall[2].attributes('disabled')).toBeUndefined();
  });

  it('uninstalls only after confirmation and then rereads the inventory', async () => {
    const wrapper = render();
    await scanHomebrew(wrapper);
    await wrapper
      .findAll('button')
      .filter(button => button.text() === 'Uninstall')[0]
      .trigger('click');

    expect(DeveloperEnvironmentService.uninstallHomebrewPackage).not.toHaveBeenCalled();
    wrapper.findAllComponents({ name: 'MdDestructiveActionDialog' })[0].vm.$emit('confirm');
    await flushPromises();

    expect(DeveloperEnvironmentService.uninstallHomebrewPackage).toHaveBeenCalledWith('glib', 'formula');
    expect(DeveloperEnvironmentService.scanHomebrew).toHaveBeenCalledTimes(2);
    expect(wrapper.text()).toContain('glib was uninstalled and');
  });

  it('explains a package brew left in place', async () => {
    vi.mocked(DeveloperEnvironmentService.uninstallHomebrewPackage).mockResolvedValue({
      ...uninstalled,
      outcome: 'failed',
      exitCode: 1,
    });
    const wrapper = render();
    await scanHomebrew(wrapper);
    await wrapper
      .findAll('button')
      .filter(button => button.text() === 'Uninstall')[0]
      .trigger('click');
    wrapper.findAllComponents({ name: 'MdDestructiveActionDialog' })[0].vm.$emit('confirm');
    await flushPromises();

    expect(wrapper.text()).toContain('Homebrew could not uninstall glib');
  });

  it('deletes a Python environment after confirmation and removes it from the list', async () => {
    const wrapper = render();
    await wrapper.findAll('[role="tab"]')[1].trigger('click');
    await scanHomebrew(wrapper);
    await buttonByText(wrapper, 'Delete')!.trigger('click');

    expect(DeveloperEnvironmentService.deletePythonEnvironment).not.toHaveBeenCalled();
    wrapper.findAllComponents({ name: 'MdDestructiveActionDialog' })[1].vm.$emit('confirm');
    await flushPromises();

    expect(DeveloperEnvironmentService.deletePythonEnvironment).toHaveBeenCalledWith('/Users/me/.venv');
    expect(wrapper.text()).toContain('.venv was deleted and');
    expect(wrapper.text()).toContain('No virtual environments found');
    expect(DeveloperEnvironmentService.scanPythonEnvironments).toHaveBeenCalledTimes(1);
  });

  it('keeps the environment listed when the backend refuses to delete it', async () => {
    vi.mocked(DeveloperEnvironmentService.deletePythonEnvironment).mockResolvedValue({
      ...deleted,
      outcome: 'unexpectedContents',
      releasedBytes: 0,
    });
    const wrapper = render();
    await wrapper.findAll('[role="tab"]')[1].trigger('click');
    await scanHomebrew(wrapper);
    await buttonByText(wrapper, 'Delete')!.trigger('click');
    wrapper.findAllComponents({ name: 'MdDestructiveActionDialog' })[1].vm.$emit('confirm');
    await flushPromises();

    expect(wrapper.text()).toContain('contains files a virtual environment does not create');
    expect(wrapper.text()).toContain('Python 3.13.0');
  });

  it('reports an absent Homebrew installation instead of an empty list', async () => {
    vi.mocked(DeveloperEnvironmentService.scanHomebrew).mockResolvedValue({
      ...homebrew,
      supported: false,
      packages: [],
      totalBytes: 0,
    });
    const wrapper = render();
    await scanHomebrew(wrapper);

    expect(wrapper.text()).toContain('Homebrew is not installed');
  });

  it('renders Russian copy', async () => {
    const wrapper = render(ru as typeof en);
    await buttonByText(wrapper, 'Сканировать')!.trigger('click');
    await flushPromises();

    expect(wrapper.text()).toContain('Неиспользуемая зависимость');
  });
});
