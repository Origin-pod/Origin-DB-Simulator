import { ReactFlowProvider } from '@xyflow/react';
import { TopBar } from '@/components/layout/TopBar';
import { BlockPalette } from '@/components/layout/BlockPalette';
import { Canvas } from '@/components/layout/Canvas';
import { ParameterPanel } from '@/components/layout/ParameterPanel';

function App() {
  return (
    <ReactFlowProvider>
      <div className="h-screen w-screen flex flex-col overflow-hidden">
        <TopBar />
        <div className="flex-1 flex overflow-hidden">
          <BlockPalette />
          <Canvas />
          <ParameterPanel />
        </div>
      </div>
    </ReactFlowProvider>
  );
}

export default App;
