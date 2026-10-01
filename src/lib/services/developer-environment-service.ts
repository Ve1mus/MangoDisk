import { invoke } from '@tauri-apps/api/core';

import type { HomebrewInventory, PythonEnvironmentScan } from '@/lib/models/developer-environments';

export class DeveloperEnvironmentService {
  static scanHomebrew(): Promise<HomebrewInventory> {
    return invoke<HomebrewInventory>('scan_homebrew_packages');
  }

  static scanPythonEnvironments(): Promise<PythonEnvironmentScan> {
    return invoke<PythonEnvironmentScan>('scan_python_environments');
  }
}
