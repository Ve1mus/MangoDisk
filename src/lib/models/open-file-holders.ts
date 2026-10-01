export interface OpenFileHolder {
  pid: number;
  command: string;
  executablePath: string | null;
  applicationPath: string | null;
  openFileCount: number;
  samplePaths: string[];
}

export interface OpenFileHoldersResult {
  schemaVersion: number;
  /** The path that was searched; null lists every open file of the account. */
  path: string | null;
  holders: OpenFileHolder[];
  elapsedMs: number;
}

/** What the last close attempt left behind, for the notice above the list. */
export type OpenFileHolderCloseOutcome = 'closed' | 'stillRunning';
