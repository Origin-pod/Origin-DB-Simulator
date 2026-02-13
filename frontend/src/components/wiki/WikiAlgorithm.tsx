interface WikiAlgorithmProps {
  algorithm: string;
}

export function WikiAlgorithm({ algorithm }: WikiAlgorithmProps) {
  return (
    <pre className="bg-gray-50 text-gray-800 rounded-lg px-4 py-3 text-xs font-mono leading-relaxed border border-gray-200 overflow-x-auto whitespace-pre-wrap">
      {algorithm}
    </pre>
  );
}
