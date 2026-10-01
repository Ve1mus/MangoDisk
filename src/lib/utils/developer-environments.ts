import type {
  HomebrewPackage,
  HomebrewRole,
  PythonEnvironment,
  PythonEnvironmentFlag,
} from '@/lib/models/developer-environments';

/**
 * A package nobody asked for and nothing needs is the usual leftover. Packages
 * with dependents are kept apart so the user does not remove one that breaks a
 * tool they use.
 */
export function homebrewRole(item: HomebrewPackage): HomebrewRole {
  if (item.installedOnRequest) return 'requested';
  return item.requiredBy.length ? 'dependency' : 'unusedDependency';
}

export function homebrewUninstallCommand(item: HomebrewPackage): string {
  return item.kind === 'cask' ? `brew uninstall --cask ${item.name}` : `brew uninstall ${item.name}`;
}

/** Ordered by how strongly each condition suggests an environment worth reviewing. */
export function pythonEnvironmentFlags(item: PythonEnvironment): PythonEnvironmentFlag[] {
  const flags: PythonEnvironmentFlag[] = [];
  if (item.interpreterMissing) flags.push('interpreterMissing');
  if (item.inHomeRoot) flags.push('homeRoot');
  if (!item.hasProjectMarkers && !item.toolManaged) flags.push('noProject');
  if (item.toolManaged) flags.push('toolManaged');
  if (item.includeSystemSitePackages) flags.push('systemSitePackages');
  return flags;
}

export function pythonEnvironmentName(item: Pick<PythonEnvironment, 'path'>): string {
  const segments = item.path.split(/[\\/]+/u).filter(Boolean);
  const name = segments.at(-1) ?? item.path;
  const owner = segments.at(-2);
  return owner && /^\.?(?:venv|env|virtualenv)$/iu.test(name) ? `${owner}/${name}` : name;
}
