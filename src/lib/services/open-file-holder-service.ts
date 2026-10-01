import { invoke } from '@tauri-apps/api/core';

import type { ApplicationCloseBatchResult, ApplicationCloseMode } from '@/lib/models/application-close';
import type { OpenFileHoldersResult } from '@/lib/models/open-file-holders';

export class OpenFileHolderService {
  /** A null path lists every application of the account that holds any file open. */
  static find(path: string | null): Promise<OpenFileHoldersResult> {
    return invoke<OpenFileHoldersResult>('find_open_file_holders', { path });
  }

  static close(
    path: string | null,
    executablePaths: string[],
    mode: ApplicationCloseMode
  ): Promise<ApplicationCloseBatchResult> {
    return invoke<ApplicationCloseBatchResult>('close_open_file_holders', { path, executablePaths, mode });
  }
}
