import { describe, expect, it } from 'vitest';

import type { OpenFileHolder } from '@/lib/models/open-file-holders';

import { groupHoldersByApplication, holderDisplayName, holderIsHelper } from './open-file-holders';

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

describe('grouping open file holders by application', () => {
  it('puts helpers under their application and ranks applications by open files', () => {
    const groups = groupHoldersByApplication([
      holder({
        pid: 1,
        command: 'WhatsApp',
        executablePath: '/Applications/WhatsApp.app/Contents/MacOS/WhatsApp',
        openFileCount: 2,
      }),
      holder({ pid: 2, command: 'ServiceExtension', openFileCount: 7 }),
      holder({
        pid: 3,
        command: 'node',
        applicationPath: null,
        executablePath: '/usr/local/bin/node',
        openFileCount: 5,
      }),
    ]);

    expect(groups.map(group => group.name)).toEqual(['WhatsApp', 'node']);
    expect(groups[0].openFileCount).toBe(9);
    expect(groups[0].holders.map(item => item.pid)).toEqual([2, 1]);
    expect(groups[0].executablePaths).toHaveLength(2);
    expect(groups[1].applicationPath).toBeNull();
  });

  it('keeps a process with no known image as its own group that cannot be closed', () => {
    const [group] = groupHoldersByApplication([
      holder({ pid: 9, command: 'mystery', applicationPath: null, executablePath: null }),
    ]);

    expect(group.key).toBe('pid:9');
    expect(group.executablePaths).toEqual([]);
  });

  it('lists a shared executable once', () => {
    const [group] = groupHoldersByApplication([holder({ pid: 1 }), holder({ pid: 2 })]);

    expect(group.executablePaths).toHaveLength(1);
    expect(group.holders).toHaveLength(2);
  });
});
