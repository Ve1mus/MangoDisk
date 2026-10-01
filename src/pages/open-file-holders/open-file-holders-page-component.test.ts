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

const serviceExtension =
  '/Applications/WhatsApp.app/Contents/PlugIns/ServiceExtension.appex/Contents/MacOS/ServiceExtension';
const found: OpenFileHoldersResult = {
  schemaVersion: 1,
  path: '/Users/me/Library/Containers/net.whatsapp.WhatsApp',
  elapsedMs: 4,
  holders: [
    {
      pid: 3544,
      command: 'ServiceExtension',
      executablePath: serviceExtension,
      applicationPath: '/Applications/WhatsApp.app',
      openFileCount: 3,
      samplePaths: ['/Users/me/Library/Containers/net.whatsapp.WhatsApp/Data/x.sqlite'],
    },
  ],
};
const everything: OpenFileHoldersResult = {
  ...found,
  path: null,
  holders: [
    ...found.holders,
    {
      pid: 100,
      command: 'WhatsApp',
      executablePath: '/Applications/WhatsApp.app/Contents/MacOS/WhatsApp',
      applicationPath: '/Applications/WhatsApp.app',
      openFileCount: 25,
      samplePaths: ['/a'],
    },
    {
      pid: 200,
      command: 'node',
      executablePath: '/usr/local/bin/node',
      applicationPath: null,
      openFileCount: 4,
      samplePaths: [],
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
      plugins: [createPinia(), createI18n({ legacy: false, locale: 'en-US', messages: { 'en-US': en } })],
      stubs: { MdIcon: true, MdIconAction: true, MdConfirmDialog: true, MdNativeFileIcon: true },
    },
  });
}

const buttonByText = (wrapper: ReturnType<typeof render>, text: string) =>
  wrapper.findAll('button').find(button => button.text() === text);

async function searchPath(wrapper: ReturnType<typeof render>) {
  await wrapper.find('input').setValue(found.path!);
  await wrapper.find('form').trigger('submit');
  await flushPromises();
}

describe('open files page', () => {
  it('starts with a Search button and searches nothing until it is pressed', async () => {
    const wrapper = render();
    await flushPromises();

    expect(OpenFileHolderService.find).not.toHaveBeenCalled();
    expect(wrapper.text()).toContain('See which apps keep files open');

    await wrapper
      .findAll('button')
      .filter(button => button.text() === 'Search')[1]
      .trigger('click');
    await flushPromises();

    expect(OpenFileHolderService.find).toHaveBeenCalledWith(null);
  });

  it('groups every process of an application under it, as Activity Monitor does', async () => {
    vi.mocked(OpenFileHolderService.find).mockResolvedValue(everything);
    const wrapper = render();
    await wrapper.find('form').trigger('submit');
    await flushPromises();

    const text = wrapper.text();
    expect(text).toContain('2 apps · 3 processes · 32 open files');
    expect(text).toContain('WhatsApp');
    expect(text).toContain('node');
    expect(text).not.toContain('ServiceExtension');

    await wrapper.findAll('[aria-expanded]')[0].trigger('click');

    expect(wrapper.text()).toContain('ServiceExtension');
    expect(wrapper.text()).toContain('PID 3544');
    expect(wrapper.text()).toContain('x.sqlite');
  });

  it('narrows the search to a path when one is entered', async () => {
    const wrapper = render();
    await searchPath(wrapper);

    expect(OpenFileHolderService.find).toHaveBeenCalledWith(found.path);
    expect(wrapper.text()).toContain(`Holding ${found.path}`);
    expect(wrapper.text()).toContain('WhatsApp');
  });

  it('closes every executable of the application only after confirmation, then rereads the list', async () => {
    vi.mocked(OpenFileHolderService.find).mockResolvedValue(everything);
    vi.mocked(OpenFileHolderService.close).mockResolvedValue({
      mode: 'force',
      matchedProcessCount: 2,
      requestedProcessCount: 2,
      remainingProcessCount: 0,
      failedTargetCount: 0,
      targets: [],
      elapsedMs: 1,
    });
    const wrapper = render();
    await wrapper.find('form').trigger('submit');
    await flushPromises();
    await buttonByText(wrapper, 'Force close')!.trigger('click');

    expect(OpenFileHolderService.close).not.toHaveBeenCalled();
    wrapper.findComponent({ name: 'MdConfirmDialog' }).vm.$emit('confirm');
    await flushPromises();

    expect(OpenFileHolderService.close).toHaveBeenCalledWith(
      null,
      [serviceExtension, '/Applications/WhatsApp.app/Contents/MacOS/WhatsApp'],
      'force'
    );
    expect(OpenFileHolderService.find).toHaveBeenCalledTimes(2);
    expect(wrapper.text()).toContain('The process has stopped');
  });

  it('explains an empty result', async () => {
    vi.mocked(OpenFileHolderService.find).mockResolvedValue({ ...found, holders: [] });
    const wrapper = render();
    await searchPath(wrapper);

    expect(wrapper.text()).toContain('Nothing is holding this path');
  });

  it('explains an empty system listing differently from an empty path search', async () => {
    vi.mocked(OpenFileHolderService.find).mockResolvedValue({ ...everything, holders: [] });
    const wrapper = render();
    await wrapper.find('form').trigger('submit');
    await flushPromises();

    expect(wrapper.text()).toContain('No open files found');
  });
});
