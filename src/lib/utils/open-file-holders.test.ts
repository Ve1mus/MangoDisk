import { describe, expect, it } from 'vitest';

import type { OpenFileHolder } from '@/lib/models/open-file-holders';

import { holderDisplayName, holderIsHelper } from './open-file-holders';

const holder = (overrides: Partial<OpenFileHolder> = {}): OpenFileHolder => ({
  pid: 1,
  command: 'ServiceExtension',
  executablePath: '/Applications/WhatsApp.app/Contents/PlugIns/ServiceExtension.appex/Contents/MacOS/ServiceExtension',
  applicationPath: '/Applications/WhatsApp.app',
  openFileCount: 3,
  samplePaths: [],
  ...overrides,
});

describe('open file holder presentation', () => {
  it('names a helper after the application that owns it', () => {
    expect(holderDisplayName(holder())).toBe('WhatsApp');
    expect(holderIsHelper(holder())).toBe(true);
  });

  it('keeps the command name for a process outside an application bundle', () => {
    const plain = holder({ command: 'node', applicationPath: null, executablePath: '/usr/local/bin/node' });

    expect(holderDisplayName(plain)).toBe('node');
    expect(holderIsHelper(plain)).toBe(false);
  });

  it('does not call the main application executable a helper', () => {
    expect(holderIsHelper(holder({ command: 'WhatsApp' }))).toBe(false);
  });
});
