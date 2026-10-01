import { describe, expect, it } from 'vitest';

import type { HomebrewPackage, PythonEnvironment } from '@/lib/models/developer-environments';

import {
  homebrewRole,
  homebrewUninstallCommand,
  pythonEnvironmentFlags,
  pythonEnvironmentName,
} from './developer-environments';

const formula = (overrides: Partial<HomebrewPackage> = {}): HomebrewPackage => ({
  name: 'wget',
  kind: 'formula',
  versions: ['1.0'],
  bytes: 10,
  installedAtMs: null,
  installedOnRequest: true,
  requiredBy: [],
  dependencies: [],
  path: '/opt/homebrew/Cellar/wget',
  ...overrides,
});

const environment = (overrides: Partial<PythonEnvironment> = {}): PythonEnvironment => ({
  path: '/Users/me/app/.venv',
  pythonVersion: '3.12.1',
  interpreterMissing: false,
  includeSystemSitePackages: false,
  hasProjectMarkers: true,
  inHomeRoot: false,
  toolManaged: false,
  bytes: 1,
  fileCount: 1,
  createdAtMs: null,
  ...overrides,
});

describe('developer environment presentation', () => {
  it('separates requested packages, used dependencies, and removable leftovers', () => {
    expect(homebrewRole(formula())).toBe('requested');
    expect(homebrewRole(formula({ installedOnRequest: false, requiredBy: ['app'] }))).toBe('dependency');
    expect(homebrewRole(formula({ installedOnRequest: false }))).toBe('unusedDependency');
  });

  it('builds the uninstall command for the package kind', () => {
    expect(homebrewUninstallCommand(formula())).toBe('brew uninstall wget');
    expect(homebrewUninstallCommand(formula({ kind: 'cask', name: 'claude-code' }))).toBe(
      'brew uninstall --cask claude-code'
    );
  });

  it('flags broken, global, and project-less environments but not healthy ones', () => {
    expect(pythonEnvironmentFlags(environment())).toEqual([]);
    expect(
      pythonEnvironmentFlags(
        environment({
          interpreterMissing: true,
          inHomeRoot: true,
          hasProjectMarkers: false,
          includeSystemSitePackages: true,
        })
      )
    ).toEqual(['interpreterMissing', 'homeRoot', 'noProject', 'systemSitePackages']);
  });

  it('does not call a tool-managed environment project-less', () => {
    expect(pythonEnvironmentFlags(environment({ hasProjectMarkers: false, toolManaged: true }))).toEqual([
      'toolManaged',
    ]);
  });

  it('names an environment after its project folder when the folder is generic', () => {
    expect(pythonEnvironmentName(environment())).toBe('app/.venv');
    expect(pythonEnvironmentName(environment({ path: '/Users/me/.platformio/penv' }))).toBe('penv');
    expect(pythonEnvironmentName(environment({ path: 'C:\\work\\api\\venv' }))).toBe('api/venv');
  });
});
