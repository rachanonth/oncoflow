export interface DatabaseIssue {
  code: string;
  title: string;
  message: string;
}

export interface StartupStatus {
  databaseReady: boolean;
  databaseLocation: string;
  issue: DatabaseIssue | null;
}

export interface BackupResult {
  location: string;
  manifestLocation: string;
  fileName: string;
  createdAt: string;
  schemaVersion: number;
  applicationVersion: string;
  integrityCheck: string;
  foreignKeyViolations: number;
  sha256: string;
  sizeBytes: number;
}

export interface RestorePreflight {
  location: string;
  fileName: string;
  schemaVersion: number;
  supportedSchemaVersion: number;
  requiresMigration: boolean;
  createdAt: string | null;
  backupApplicationVersion: string | null;
  integrityCheck: string;
  foreignKeyViolations: number;
  sha256: string;
  sizeBytes: number;
  confirmationToken: string;
}

export interface RestoreInput {
  backupPath: string;
  expectedSha256: string;
  confirmationToken: string;
  confirmed: boolean;
}

export interface RestoreResult {
  restoredSchemaVersion: number;
  migratedFromSchemaVersion: number | null;
  recoveryBackupLocation: string;
  recoveryBackupSha256: string;
  restoredBackupSha256: string;
  sessionCleared: boolean;
  restartRequired: boolean;
}

export interface Diagnostics {
  applicationName: string;
  applicationVersion: string;
  schemaVersion: number;
  clinicalRulesetVersion: string;
  labelTemplateVersion: string;
  labelRendererVersion: string;
  databaseLocation: string;
  databaseSizeBytes: number;
  integrityCheck: string;
  foreignKeyViolations: number;
  lastBackupAt: string | null;
  platform: string;
  automaticBackupPolicy: string;
}

export interface PrinterQueueStatus {
  configuredQueue: string | null;
  available: boolean;
  installedQueueCount: number;
  physicalOutputConfirmed: boolean;
}
