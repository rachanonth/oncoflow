export interface HealthStatus {
  backendRunning: boolean;
  databaseConnected: boolean;
  schemaVersion: number;
}
