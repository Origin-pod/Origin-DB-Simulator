import { useState, useEffect, useCallback } from 'react';
import {
  Sparkles,
  MousePointerClick,
  Cable,
  Settings,
  Play,
  BarChart3,
  ArrowRight,
  X,
  LayoutTemplate,
} from 'lucide-react';
import { Button } from '@/components/ui/Button';

// ---------------------------------------------------------------------------
// Storage key
// ---------------------------------------------------------------------------

const STORAGE_KEY = 'db-sim-onboarding-complete';

function isOnboardingComplete(): boolean {
  try {
    return localStorage.getItem(STORAGE_KEY) === 'true';
  } catch {
    return false;
  }
}

function markOnboardingComplete(): void {
  try {
    localStorage.setItem(STORAGE_KEY, 'true');
  } catch {
    // ignore
  }
}

// ---------------------------------------------------------------------------
// Steps
// ---------------------------------------------------------------------------

interface TutorialStep {
  title: string;
  description: string;
  icon: React.ReactNode;
  highlight?: string; // CSS selector hint (not used for actual DOM targeting, just descriptive)
}

const STEPS: TutorialStep[] = [
  {
    title: 'Welcome to DB Simulator!',
    description:
      'Design databases visually, run workloads, and compare performance instantly. This quick tour will show you the basics.',
    icon: <Sparkles className="w-6 h-6" />,
  },
  {
    title: 'Block Palette',
    description:
      'On the left you\'ll find building blocks — storage engines, indexes, buffers, and more. Each represents a real database component.',
    icon: <LayoutTemplate className="w-6 h-6" />,
    highlight: 'left-palette',
  },
  {
    title: 'Drag & Drop',
    description:
      'Drag blocks from the palette onto the canvas. Position them to build your database architecture.',
    icon: <MousePointerClick className="w-6 h-6" />,
    highlight: 'canvas',
  },
  {
    title: 'Connect Blocks',
    description:
      'Click an output port (right side) and drag to an input port (left side) to create data flow connections between blocks.',
    icon: <Cable className="w-6 h-6" />,
    highlight: 'canvas',
  },
  {
    title: 'Configure Parameters',
    description:
      'Click any block to select it. The right panel shows its configuration — page size, fanout, buffer size, and more.',
    icon: <Settings className="w-6 h-6" />,
    highlight: 'right-panel',
  },
  {
    title: 'Run a Workload',
    description:
      'Define a workload (mix of reads, writes, scans) then click Run. The simulator will execute your design and generate performance metrics.',
    icon: <Play className="w-6 h-6" />,
    highlight: 'top-bar',
  },
  {
    title: 'Analyze & Compare',
    description:
      'After running, the metrics dashboard shows throughput, latency, and per-block breakdown. Create multiple designs and compare them side-by-side!',
    icon: <BarChart3 className="w-6 h-6" />,
    highlight: 'bottom',
  },
];

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

interface OnboardingProps {
  onOpenTemplates: () => void;
}

export function OnboardingTutorial({ onOpenTemplates }: OnboardingProps) {
  const [visible, setVisible] = useState(false);
  const [step, setStep] = useState(0);

  useEffect(() => {
    if (!isOnboardingComplete()) {
      // Small delay so the app renders first
      const t = setTimeout(() => setVisible(true), 500);
      return () => clearTimeout(t);
    }
  }, []);

  const handleNext = useCallback(() => {
    if (step < STEPS.length - 1) {
      setStep(step + 1);
    } else {
      // Finished
      markOnboardingComplete();
      setVisible(false);
    }
  }, [step]);

  const handleSkip = useCallback(() => {
    markOnboardingComplete();
    setVisible(false);
  }, []);

  const handleFinishWithTemplates = useCallback(() => {
    markOnboardingComplete();
    setVisible(false);
    onOpenTemplates();
  }, [onOpenTemplates]);

  if (!visible) return null;

  const current = STEPS[step];
  const isLast = step === STEPS.length - 1;
  const isFirst = step === 0;

  return (
    <div className="fixed inset-0 z-[60] flex items-center justify-center">
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/50 backdrop-blur-sm" />

      {/* Card */}
      <div className="relative bg-white rounded-2xl shadow-2xl w-full max-w-md overflow-hidden">
        {/* Progress bar */}
        <div className="h-1 bg-gray-100">
          <div
            className="h-full bg-primary-500 transition-all duration-300"
            style={{ width: `${((step + 1) / STEPS.length) * 100}%` }}
          />
        </div>

        {/* Close / skip */}
        <button
          onClick={handleSkip}
          className="absolute top-4 right-4 p-1 text-gray-400 hover:text-gray-600 z-10"
          title="Skip tutorial"
        >
          <X className="w-4 h-4" />
        </button>

        {/* Content */}
        <div className="px-8 pt-8 pb-6 text-center">
          {/* Icon */}
          <div className="w-14 h-14 mx-auto mb-4 rounded-2xl bg-primary-50 text-primary-500 flex items-center justify-center">
            {current.icon}
          </div>

          {/* Step indicator */}
          <p className="text-xs text-gray-400 mb-2">
            {step + 1} of {STEPS.length}
          </p>

          <h2 className="text-lg font-semibold text-gray-900 mb-2">
            {current.title}
          </h2>
          <p className="text-sm text-gray-600 leading-relaxed">
            {current.description}
          </p>
        </div>

        {/* Dots */}
        <div className="flex items-center justify-center gap-1.5 pb-4">
          {STEPS.map((_, i) => (
            <button
              key={i}
              onClick={() => setStep(i)}
              className={`w-2 h-2 rounded-full transition-colors ${
                i === step ? 'bg-primary-500' : 'bg-gray-200 hover:bg-gray-300'
              }`}
            />
          ))}
        </div>

        {/* Actions */}
        <div className="flex items-center justify-between px-6 py-4 border-t border-gray-100 bg-gray-50">
          {isFirst ? (
            <button
              onClick={handleSkip}
              className="text-xs text-gray-500 hover:text-gray-700"
            >
              Skip tutorial
            </button>
          ) : (
            <button
              onClick={() => setStep(step - 1)}
              className="text-xs text-gray-500 hover:text-gray-700"
            >
              Back
            </button>
          )}

          <div className="flex items-center gap-2">
            {isLast ? (
              <>
                <Button variant="secondary" size="sm" onClick={handleFinishWithTemplates}>
                  <LayoutTemplate className="w-4 h-4" />
                  Try a Template
                </Button>
                <Button variant="primary" size="sm" onClick={handleNext}>
                  Start Building
                </Button>
              </>
            ) : (
              <Button variant="primary" size="sm" onClick={handleNext}>
                Next
                <ArrowRight className="w-4 h-4" />
              </Button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
