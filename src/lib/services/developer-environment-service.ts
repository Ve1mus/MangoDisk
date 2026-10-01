import { invoke } from '@tauri-apps/api/core';

import type {
  HomebrewInventory,
  HomebrewPackageKind,
  HomebrewUninstallResult,
  PythonEnvironmentDeleteResult,
  PythonEnvironmentScan,
} from '@/lib/models/developer-environments';

export class DeveloperEnvironmentService {
  static scanHomebrew(): Promise<HomebrewInventory> {
    return invoke<HomebrewInventory>('scan_homebrew_packages');
  }

  static scanPythonEnvironments(): Promise<PythonEnvironmentScan> {
    return invoke<PythonEnvironmentScan>('scan_python_environments');
  }

  static uninstallHomebrewPackage(name: string, kind: HomebrewPackageKind): Promise<HomebrewUninstallResult> {
    return invoke<HomebrewUninstallResult>('uninstall_homebrew_package', { name, kind });
  }

  static deletePythonEnvironment(path: string): Promise<PythonEnvironmentDeleteResult> {
    return invoke<PythonEnvironmentDeleteResult>('delete_python_environment', { path });
  }
}
