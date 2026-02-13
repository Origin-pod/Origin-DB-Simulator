import { useState, useCallback, useEffect } from 'react';
import { ReactFlowProvider } from '@xyflow/react';
import { TopBar } from '@/components/layout/TopBar';
import { useDesignStore } from '@/stores/designStore';
import { useExecutionStore } from '@/stores/executionStore';
import { loadWASM } from '@/wasm/loader';
import { hydrateBlockRegistry } from '@/wasm/hydrate';
import { DesignTabs } from '@/components/layout/DesignTabs';
import { BlockPalette } from '@/components/layout/BlockPalette';
import { Canvas } from '@/components/layout/Canvas';
import { ParameterPanel } from '@/components/layout/ParameterPanel';
import { WorkloadEditor } from '@/components/workload/WorkloadEditor';
import { ExecutionOverlay } from '@/components/execution/ExecutionOverlay';
import { MetricsDashboard } from '@/components/metrics/MetricsDashboard';
import { ComparisonView } from '@/components/comparison/ComparisonView';
import { TemplateModal } from '@/components/templates/TemplateModal';
import { OnboardingTutorial } from '@/components/onboarding/OnboardingTutorial';
import { ToastContainer } from '@/components/ui/ToastContainer';

function App() {
  const [comparisonOpen, setComparisonOpen] = useState(false);
  const [templatesOpen, setTemplatesOpen] = useState(false);

  const openTemplates = useCallback(() => setTemplatesOpen(true), []);

  // Hydrate from localStorage and attempt WASM load on first mount
  useEffect(() => {
    useDesignStore.getState().hydrate();

    // Attempt to load WASM module (non-blocking, falls back to mock)
    loadWASM().then((loaded) => {
      useExecutionStore.getState().refreshEngineType();
      if (loaded) {
        hydrateBlockRegistry();
      }
    });
  }, []);

  return (
    <ReactFlowProvider>
      <div className="h-screen w-screen flex flex-col overflow-hidden">
        <TopBar
          onOpenComparison={() => setComparisonOpen(true)}
          onOpenTemplates={openTemplates}
        />
        <DesignTabs />
        <div className="flex-1 flex overflow-hidden">
          <BlockPalette />
          <Canvas />
          <ParameterPanel />
        </div>
        <MetricsDashboard />
      </div>
      <WorkloadEditor />
      <ExecutionOverlay />
      <ComparisonView
        open={comparisonOpen}
        onClose={() => setComparisonOpen(false)}
      />
      <TemplateModal
        open={templatesOpen}
        onClose={() => setTemplatesOpen(false)}
      />
      <OnboardingTutorial onOpenTemplates={openTemplates} />
      <ToastContainer />
    </ReactFlowProvider>
  );
}

export default App;
