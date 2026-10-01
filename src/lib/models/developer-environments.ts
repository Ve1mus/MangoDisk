export type HomebrewPackageKind = 'formula' | 'cask';

export interface HomebrewPackage {
  name: string;
  kind: HomebrewPackageKind;
  versions: string[];
  bytes: number;
  installedAtMs: number | null;
  installedOnRequest: boolean;
  requiredBy: string[];
  /** Runtime dependencies the package declares, as installed formula names. */
  dependencies: string[];
  path: string;
}

export interface HomebrewInventory {
  schemaVersion: number;
  supported: boolean;
  prefix: string | null;
  packages: HomebrewPackage[];
  totalBytes: number;
}

export interface PythonEnvironment {
  path: string;
  pythonVersion: string | null;
  interpreterMissing: boolean;
  includeSystemSitePackages: boolean;
  hasProjectMarkers: boolean;
  inHomeRoot: boolean;
  toolManaged: boolean;
  bytes: number;
  fileCount: number;
  createdAtMs: number | null;
}

export interface PythonEnvironmentScan {
  schemaVersion: number;
  environments: PythonEnvironment[];
  totalBytes: number;
  complete: boolean;
  elapsedMs: number;
}

/** Why a Homebrew package is installed, from the user's point of view. */
export type HomebrewRole = 'requested' | 'dependency' | 'unusedDependency';

export type PythonEnvironmentFlag =
  'interpreterMissing' | 'homeRoot' | 'noProject' | 'toolManaged' | 'systemSitePackages';

export type HomebrewUninstallOutcome = 'removed' | 'notInstalled' | 'stillRequired' | 'stillInstalled' | 'failed';

export interface HomebrewUninstallResult {
  schemaVersion: number;
  name: string;
  kind: HomebrewPackageKind;
  outcome: HomebrewUninstallOutcome;
  releasedBytes: number;
  requiredBy: string[];
  exitCode: number | null;
}

export type PythonEnvironmentDeleteOutcome =
  'removed' | 'notFound' | 'notAnEnvironment' | 'outsideScope' | 'unexpectedContents' | 'failed';

export interface PythonEnvironmentDeleteResult {
  schemaVersion: number;
  path: string;
  outcome: PythonEnvironmentDeleteOutcome;
  releasedBytes: number;
  removedFileCount: number;
}

/** The last removal, for the notice above the list. Rendered from codes, never from backend text. */
export type DeveloperEnvironmentNotice =
  | { kind: 'homebrew'; name: string; outcome: HomebrewUninstallOutcome; requiredBy: string[]; releasedBytes: number }
  | { kind: 'python'; name: string; outcome: PythonEnvironmentDeleteOutcome; releasedBytes: number };
