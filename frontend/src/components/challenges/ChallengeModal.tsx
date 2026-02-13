import { X, GraduationCap, Clock, CheckCircle2 } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { useChallengeStore } from '@/stores/challengeStore';
import { getChallenges, DIFFICULTY_COLORS, DIFFICULTY_LABELS } from '@/data/challenges';

export function ChallengeModal() {
  const { modalOpen, closeModal, startChallenge, completedChallenges } = useChallengeStore();

  if (!modalOpen) return null;

  const challenges = getChallenges();

  return (
    <>
      {/* Backdrop */}
      <div className="fixed inset-0 bg-black/30 z-50" onClick={closeModal} />

      {/* Modal */}
      <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
        <div
          className="bg-white rounded-xl shadow-xl w-full max-w-2xl max-h-[80vh] flex flex-col"
          onClick={(e) => e.stopPropagation()}
        >
          {/* Header */}
          <div className="flex items-center justify-between px-6 py-4 border-b border-gray-200">
            <div className="flex items-center gap-2">
              <GraduationCap className="w-5 h-5 text-blue-600" />
              <h2 className="text-lg font-semibold text-gray-900">
                Learning Challenges
              </h2>
            </div>
            <button
              onClick={closeModal}
              className="text-gray-400 hover:text-gray-600"
            >
              <X className="w-5 h-5" />
            </button>
          </div>

          <p className="px-6 py-3 text-sm text-gray-600 border-b border-gray-100">
            Guided experiments that teach database concepts through hands-on exploration.
            Each challenge walks you through a concept step by step.
          </p>

          {/* Challenge list */}
          <div className="flex-1 overflow-y-auto px-6 py-4 space-y-3">
            {challenges.map((challenge) => {
              const isCompleted = completedChallenges.includes(challenge.id);
              const diffColor = DIFFICULTY_COLORS[challenge.difficulty];

              return (
                <div
                  key={challenge.id}
                  className={`border rounded-xl p-4 transition-colors ${
                    isCompleted
                      ? 'border-green-200 bg-green-50/50'
                      : 'border-gray-200 hover:border-blue-300 hover:bg-blue-50/30'
                  }`}
                >
                  <div className="flex items-start justify-between gap-3">
                    <div className="flex-1">
                      <div className="flex items-center gap-2 mb-1">
                        {isCompleted && (
                          <CheckCircle2 className="w-4 h-4 text-green-500 flex-shrink-0" />
                        )}
                        <h3 className="text-sm font-semibold text-gray-900">
                          {challenge.title}
                        </h3>
                      </div>
                      <p className="text-xs text-gray-600 mb-2">
                        {challenge.subtitle}
                      </p>
                      <div className="flex items-center gap-3">
                        <span
                          className="text-[10px] font-semibold px-2 py-0.5 rounded-full"
                          style={{
                            backgroundColor: `${diffColor}20`,
                            color: diffColor,
                          }}
                        >
                          {DIFFICULTY_LABELS[challenge.difficulty]}
                        </span>
                        <span className="flex items-center gap-1 text-[10px] text-gray-400">
                          <Clock className="w-3 h-3" />
                          {challenge.estimatedMinutes} min
                        </span>
                        <span className="text-[10px] text-gray-400">
                          {challenge.steps.length} steps
                        </span>
                      </div>
                      <div className="flex flex-wrap gap-1 mt-2">
                        {challenge.concepts.map((c) => (
                          <span
                            key={c}
                            className="text-[10px] bg-gray-100 text-gray-600 px-1.5 py-0.5 rounded"
                          >
                            {c}
                          </span>
                        ))}
                      </div>
                    </div>
                    <Button
                      variant={isCompleted ? 'ghost' : 'primary'}
                      size="sm"
                      onClick={() => startChallenge(challenge.id)}
                      className="flex-shrink-0"
                    >
                      {isCompleted ? 'Replay' : 'Start'}
                    </Button>
                  </div>
                </div>
              );
            })}
          </div>

          {/* Footer */}
          <div className="px-6 py-3 border-t border-gray-200 bg-gray-50 rounded-b-xl flex items-center justify-between">
            <span className="text-xs text-gray-400">
              {completedChallenges.length} / {challenges.length} completed
            </span>
            <Button variant="ghost" size="sm" onClick={closeModal}>
              Close
            </Button>
          </div>
        </div>
      </div>
    </>
  );
}
