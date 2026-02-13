import type { BlockMetricInfo } from '@/types/blocks';

interface WikiMetricsProps {
  metrics: BlockMetricInfo[];
}

const TYPE_COLORS: Record<string, string> = {
  Counter: 'bg-blue-100 text-blue-700',
  Gauge: 'bg-green-100 text-green-700',
  Histogram: 'bg-purple-100 text-purple-700',
  Timing: 'bg-amber-100 text-amber-700',
};

export function WikiMetrics({ metrics }: WikiMetricsProps) {
  return (
    <div className="overflow-x-auto">
      <table className="w-full text-xs">
        <thead>
          <tr className="border-b border-gray-200">
            <th className="text-left py-2 pr-3 font-medium text-gray-500">Metric</th>
            <th className="text-left py-2 pr-3 font-medium text-gray-500">Type</th>
            <th className="text-left py-2 pr-3 font-medium text-gray-500">Unit</th>
            <th className="text-left py-2 font-medium text-gray-500">Description</th>
          </tr>
        </thead>
        <tbody>
          {metrics.map((m) => (
            <tr key={m.id} className="border-b border-gray-100 last:border-0">
              <td className="py-2 pr-3">
                <span className="font-mono text-gray-800 bg-gray-100 px-1.5 py-0.5 rounded">
                  {m.name}
                </span>
              </td>
              <td className="py-2 pr-3">
                <span className={`px-1.5 py-0.5 rounded text-[10px] font-medium ${TYPE_COLORS[m.type] ?? 'bg-gray-100 text-gray-600'}`}>
                  {m.type}
                </span>
              </td>
              <td className="py-2 pr-3 text-gray-500">{m.unit || '—'}</td>
              <td className="py-2 text-gray-600">{m.description}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
