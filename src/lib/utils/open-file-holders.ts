import type { OpenFileHolder } from '@/lib/models/open-file-holders';

/** Prefers the owning application's name; helper processes are otherwise named after their binary. */
export function holderDisplayName(holder: OpenFileHolder): string {
  const bundle = holder.applicationPath
    ?.split(/[\\/]+/u)
    .filter(Boolean)
    .at(-1);
  return bundle?.replace(/\.app$/iu, '') || holder.command;
}

/** A helper runs inside an app bundle under a binary name that differs from the bundle's. */
export function holderIsHelper(holder: OpenFileHolder): boolean {
  const name = holderDisplayName(holder);
  return Boolean(holder.applicationPath) && holder.command !== name;
}

/** One application and every process of it that holds files open, as Activity Monitor groups them. */
export interface OpenFileApplication {
  key: string;
  name: string;
  applicationPath: string | null;
  /** Distinct executables, which is how the backend identifies the processes to close. */
  executablePaths: string[];
  holders: OpenFileHolder[];
  openFileCount: number;
}

/**
 * Groups processes under their application bundle, or under their executable when they run
 * outside a bundle, so helpers do not appear as unrelated rows. Largest holders first.
 */
export function groupHoldersByApplication(holders: readonly OpenFileHolder[]): OpenFileApplication[] {
  const groups = new Map<string, OpenFileApplication>();
  for (const holder of holders) {
    const key = holder.applicationPath ?? holder.executablePath ?? `pid:${holder.pid}`;
    let group = groups.get(key);
    if (!group) {
      group = {
        key,
        name: holderDisplayName(holder),
        applicationPath: holder.applicationPath,
        executablePaths: [],
        holders: [],
        openFileCount: 0,
      };
      groups.set(key, group);
    }
    group.holders.push(holder);
    group.openFileCount += holder.openFileCount;
    if (holder.executablePath && !group.executablePaths.includes(holder.executablePath)) {
      group.executablePaths.push(holder.executablePath);
    }
  }
  const applications = [...groups.values()];
  for (const application of applications) {
    application.holders.sort((left, right) => right.openFileCount - left.openFileCount || left.pid - right.pid);
  }
  return applications.sort(
    (left, right) => right.openFileCount - left.openFileCount || left.name.localeCompare(right.name)
  );
}
