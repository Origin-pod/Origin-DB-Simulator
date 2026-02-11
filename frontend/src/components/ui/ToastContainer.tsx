import { useEffect, useState } from 'react';
import {
  CheckCircle2,
  XCircle,
  AlertTriangle,
  Info,
  X,
} from 'lucide-react';
import { useToastStore, type Toast, type ToastType } from '@/stores/toastStore';

// ---------------------------------------------------------------------------
// Style maps
// ---------------------------------------------------------------------------

const TYPE_STYLES: Record<ToastType, { bg: string; border: string; icon: React.ReactNode; text: string }> = {
  success: {
    bg: 'bg-green-50',
    border: 'border-green-200',
    icon: <CheckCircle2 className="w-4 h-4 text-green-500 flex-shrink-0" />,
    text: 'text-green-800',
  },
  error: {
    bg: 'bg-red-50',
    border: 'border-red-200',
    icon: <XCircle className="w-4 h-4 text-red-500 flex-shrink-0" />,
    text: 'text-red-800',
  },
  warning: {
    bg: 'bg-amber-50',
    border: 'border-amber-200',
    icon: <AlertTriangle className="w-4 h-4 text-amber-500 flex-shrink-0" />,
    text: 'text-amber-800',
  },
  info: {
    bg: 'bg-blue-50',
    border: 'border-blue-200',
    icon: <Info className="w-4 h-4 text-blue-500 flex-shrink-0" />,
    text: 'text-blue-800',
  },
};

// ---------------------------------------------------------------------------
// Single toast item with enter/exit animation
// ---------------------------------------------------------------------------

function ToastItem({ toast: t, onRemove }: { toast: Toast; onRemove: (id: string) => void }) {
  const [show, setShow] = useState(false);
  const style = TYPE_STYLES[t.type];

  useEffect(() => {
    // Trigger enter animation
    const frame = requestAnimationFrame(() => setShow(true));
    return () => cancelAnimationFrame(frame);
  }, []);

  const handleRemove = () => {
    setShow(false);
    setTimeout(() => onRemove(t.id), 150);
  };

  return (
    <div
      className={`
        flex items-start gap-2 px-4 py-3 rounded-lg border shadow-lg
        transition-all duration-150
        ${style.bg} ${style.border}
        ${show ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-2'}
      `}
    >
      {style.icon}
      <p className={`text-sm flex-1 ${style.text}`}>{t.message}</p>
      {t.action && (
        <button
          onClick={() => { t.action!.onClick(); handleRemove(); }}
          className={`text-xs font-semibold underline ${style.text} hover:opacity-70`}
        >
          {t.action.label}
        </button>
      )}
      <button onClick={handleRemove} className="p-0.5 text-gray-400 hover:text-gray-600">
        <X className="w-3.5 h-3.5" />
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Container
// ---------------------------------------------------------------------------

export function ToastContainer() {
  const { toasts, removeToast } = useToastStore();

  if (toasts.length === 0) return null;

  return (
    <div className="fixed top-4 right-4 z-[70] flex flex-col gap-2 w-80">
      {toasts.map((t) => (
        <ToastItem key={t.id} toast={t} onRemove={removeToast} />
      ))}
    </div>
  );
}
