import { useEffect } from 'react';
import {
  GraduationCap,
  ChevronRight,
  CheckCircle2,
  Lightbulb,
  X,
  Play,
  ArrowRight,
} from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { useChallengeStore } from '@/stores/challengeStore';
import { useExecutionStore } from '@/stores/executionStore';
import { DIFFICULTY_COLORS } from '@/data/challenges';

export function ChallengeRunner() {
  const {
    activeChallenge,
    currentStepIndex,
    stepCompleted,
    showPayoff,
    verifyStep,
    nextStep,
    exitChallenge,
    dismissPayoff,
    completedChallenges,
  } = useChallengeStore();

  const { status } = useExecutionStore();

  // Auto-verify when execution completes
  useEffect(() => {
    if (status === 'complete' && activeChallenge && !stepCompleted) {
      verifyStep();
    }
  }, [status, activeChallenge, stepCompleted, verifyStep]);

  if (!activeChallenge) return null;

  const step = activeChallenge.steps[currentStepIndex];
  const totalSteps = activeChallenge.steps.length;
  const isLastStep = currentStepIndex === totalSteps - 1;
  const isChallengeComplete = isLastStep && stepCompleted;
  const justCompleted = completedChallenges.includes(activeChallenge.id) && isChallengeComplete;
  const diffColor = DIFFICULTY_COLORS[activeChallenge.difficulty];
  const progressPct = ((currentStepIndex + (stepCompleted ? 1 : 0)) / totalSteps) * 100;

  return (
    <div className="fixed bottom-0 left-0 right-0 z-30 pointer-events-none">
      <div className="max-w-3xl mx-auto px-4 pb-4 pointer-events-auto">
        <div className="bg-white rounded-xl shadow-lg border border-gray-200 overflow-hidden">
          {/* Progress bar */}
          <div className="h-1 bg-gray-100">
            <div
              className="h-full bg-blue-500 transition-all duration-500"
              style={{ width: `${progressPct}%` }}
            />
          </div>

          {/* Header */}
          <div className="flex items-center justify-between px-4 py-2 border-b border-gray-100">
            <div className="flex items-center gap-2">
              <GraduationCap className="w-4 h-4" style={{ color: diffColor }} />
              <span className="text-xs font-semibold text-gray-900">
                {activeChallenge.title}
              </span>
              <span className="text-[10px] text-gray-400">
                Step {currentStepIndex + 1} of {totalSteps}
              </span>
            </div>
            <button
              onClick={exitChallenge}
              className="text-gray-400 hover:text-gray-600 p-1"
              title="Exit challenge"
            >
              <X className="w-4 h-4" />
            </button>
          </div>

          {/* Step content */}
          <div className="px-4 py-3">
            {/* Instruction */}
            <p className="text-sm text-gray-800 leading-relaxed">
              {step.instruction}
            </p>

            {step.hint && !stepCompleted && (
              <p className="text-xs text-gray-500 mt-2 italic">
                Hint: {step.hint}
              </p>
            )}

            {step.goalDescription && !stepCompleted && (
              <p className="text-xs text-blue-600 mt-2 flex items-center gap-1">
                <ChevronRight className="w-3 h-3" />
                {step.goalDescription}
              </p>
            )}

            {/* Educational payoff */}
            {showPayoff && (
              <div className="mt-3 bg-green-50 border border-green-200 rounded-lg px-3 py-2.5">
                <div className="flex items-start gap-2">
                  <Lightbulb className="w-4 h-4 text-green-600 mt-0.5 flex-shrink-0" />
                  <p className="text-xs text-green-800 leading-relaxed">
                    {step.educationalPayoff}
                  </p>
                </div>
              </div>
            )}

            {/* Challenge complete message */}
            {justCompleted && (
              <div className="mt-3 bg-blue-50 border border-blue-200 rounded-lg px-3 py-2.5">
                <div className="flex items-start gap-2">
                  <CheckCircle2 className="w-4 h-4 text-blue-600 mt-0.5 flex-shrink-0" />
                  <p className="text-xs text-blue-800 leading-relaxed">
                    Challenge complete! You can replay it anytime from the Challenges menu.
                  </p>
                </div>
              </div>
            )}
          </div>

          {/* Actions */}
          <div className="flex items-center justify-between px-4 py-2 bg-gray-50 border-t border-gray-100">
            <div className="flex items-center gap-2">
              {/* Step indicators */}
              {Array.from({ length: totalSteps }, (_, i) => (
                <div
                  key={i}
                  className={`w-2 h-2 rounded-full transition-colors ${
                    i < currentStepIndex || (i === currentStepIndex && stepCompleted)
                      ? 'bg-green-500'
                      : i === currentStepIndex
                        ? 'bg-blue-500'
                        : 'bg-gray-300'
                  }`}
                />
              ))}
            </div>

            <div className="flex items-center gap-2">
              {!stepCompleted && step.successCriteria?.type === 'run_execution' && (
                <span className="text-[10px] text-gray-400 flex items-center gap-1">
                  <Play className="w-3 h-3" />
                  Run the design to continue
                </span>
              )}

              {!stepCompleted && !step.successCriteria && (
                <Button variant="primary" size="sm" onClick={() => verifyStep()}>
                  <CheckCircle2 className="w-3.5 h-3.5" />
                  Continue
                </Button>
              )}

              {stepCompleted && showPayoff && !isChallengeComplete && (
                <Button variant="primary" size="sm" onClick={nextStep}>
                  Next Step
                  <ArrowRight className="w-3.5 h-3.5" />
                </Button>
              )}

              {stepCompleted && showPayoff && isChallengeComplete && (
                <Button variant="primary" size="sm" onClick={exitChallenge}>
                  <CheckCircle2 className="w-3.5 h-3.5" />
                  Finish
                </Button>
              )}

              {stepCompleted && !showPayoff && (
                <Button variant="ghost" size="sm" onClick={dismissPayoff}>
                  Hide insight
                </Button>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
