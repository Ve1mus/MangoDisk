// @vitest-environment happy-dom
import { flushPromises, mount } from '@vue/test-utils';
import { createPinia } from 'pinia';
import { createI18n } from 'vue-i18n';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import Page from './index.vue';
import type { OpenFileHoldersResult } from '@/lib/models/open-file-holders';
import { OpenFileHolderService } from '@/lib/services/open-file-holder-service';
import en from '@/locales/en-US.json';

vi.mock('@/lib/services/open-file-holder-service', () => ({
  OpenFileHolderService: { find: vi.fn(), close: vi.fn() },
}));
vi.mock('@/lib/services/folder-selection-service', () => ({ FolderSelectionService: { select: vi.fn() } }));
vi.mock('@/lib/services/file-manager-service', () => ({ FileManagerService: { reveal: vi.fn() } }));
vi.mock('@/lib/services/logger-service', () => ({ LoggerService: { warn: vi.fn() } }));
vi.mock('@/lib/services/operating-system-service', () => ({
  OperatingSystemService: { isMacOs: () => true, isWindows: () => false, isLinux: () => false },
}));

const found: OpenFileHoldersResult = {
  schemaVersion: 1,
  path: '/Users/me/Library/Containers/net.whatsapp.WhatsApp',
  elapsedMs: 4,
  holders: [
    {
      pid: 3544,
      command: 'ServiceExtension',
      executablePath:
        '/Applications/WhatsApp.app/Contents/PlugIns/ServiceExtension.appex/Contents/MacOS/ServiceExtension',
      applicationPath: '/Applications/WhatsApp.app',
      openFileCount: 3,
      samplePaths: ['/Users/me/Library/Containers/net.whatsapp.WhatsApp/Data/x.sqlite'],
    },
  ],
};

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(OpenFileHolderService.find).mockResolvedValue(found);
});

function render() {
  return mount(Page, {
    global: {
      plugins: [createPinia(), createI18n({ legacy: false, locale: 'test', messages: { test: en } })],
      stubs: { MdIcon: true, MdIconAction: true, MdConfirmDialog: true },
    },
  });
}

async function search(wrapper: ReturnType<typeof render>) {
  await wrapper.find('input').setValue(found.path);
  await wrapper.find('form').trigger('submit');
  await flushPromises();
}

describe('open files page', () => {
  it('lists the application that owns a helper process holding the path', async () => {
    const wrapper = render();
    await search(wrapper);

    expect(OpenFileHolderService.find).toHaveBeenCalledWith(found.path);
    const text = wrapper.text();
    expect(text).toContain('WhatsApp');
    expect(text).toContain('Helper: ServiceExtension');
    expect(text).toContain('PID 3544');
    expect(text).toContain('Open files: 3');
  });

  it('closes only after confirmation and then rereads the live list', async () => {
    vi.mocked(OpenFileHolderService.close).mockResolvedValue({
      mode: 'force',
      matchedProcessCount: 1,
      requestedProcessCount: 1,
      remainingProcessCount: 0,
      failedTargetCount: 0,
      targets: [],
      elapsedMs: 1,
    });
    const wrapper = render();
    await search(wrapper);
    const forceButton = wrapper.findAll('button').find(button => button.text() === 'Force close');
    await forceButton!.trigger('click');

    expect(OpenFileHolderService.close).not.toHaveBeenCalled();
    wrapper.findComponent({ name: 'MdConfirmDialog' }).vm.$emit('confirm');
    await flushPromises();

    expect(OpenFileHolderService.close).toHaveBeenCalledWith(found.path, found.holders[0].executablePath, 'force');
    expect(OpenFileHolderService.find).toHaveBeenCalledTimes(2);
    expect(wrapper.text()).toContain('The process has stopped');
  });

  it('explains an empty result', async () => {
    vi.mocked(OpenFileHolderService.find).mockResolvedValue({ ...found, holders: [] });
    const wrapper = render();
    await search(wrapper);

    expect(wrapper.text()).toContain('Nothing is holding this path');
  });
});
