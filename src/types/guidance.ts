export interface PageGuidanceRecord {
  pageKey: string;
  guidance: string | null;
}

export interface UpdatePageGuidanceInput {
  pageKey: string;
  guidance: string | null;
}
