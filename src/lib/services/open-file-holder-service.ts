import { invoke } from '@tauri-apps/api/core';

import type { ApplicationCloseBatchResult, ApplicationCloseMode } from '@/lib/models/application-close';
import type { OpenFileHoldersResult } from '@/lib/models/open-file-holders';

export class OpenFileHolderService {
  static find(path: string): Promise<OpenFileHoldersResult> {
    return invoke<OpenFileHoldersResult>('find_open_file_holders', { path });
  }

  static close(path: string, executablePath: string, mode: ApplicationCloseMode): Promise<ApplicationCloseBatchResult> {
    return invoke<ApplicationCloseBatchResult>('close_open_file_holder', { path, executablePath, mode });
  }
}
