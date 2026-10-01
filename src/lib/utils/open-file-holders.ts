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
