export type HomebrewPackageKind = 'formula' | 'cask';

export interface HomebrewPackage {
  name: string;
  kind: HomebrewPackageKind;
  versions: string[];
  bytes: number;
  installedAtMs: number | null;
  installedOnRequest: boolean;
  requiredBy: string[];
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
