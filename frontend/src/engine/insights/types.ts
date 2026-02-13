// ---------------------------------------------------------------------------
// Insight Engine types
// ---------------------------------------------------------------------------

export type InsightType = 'bottleneck' | 'opportunity' | 'explanation' | 'comparison';
export type InsightSeverity = 'info' | 'suggestion' | 'important';

export interface Insight {
  id: string;
  type: InsightType;
  severity: InsightSeverity;
  title: string;
  explanation: string;
  whyItMatters: string;
  suggestion?: string;
  learnMore?: {
    blockType: string;
    section: 'algorithm' | 'complexity' | 'tradeoffs' | 'useCases';
  };
  realWorldExample?: string;
}
