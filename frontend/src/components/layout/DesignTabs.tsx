import { useState, useRef, useEffect, useCallback } from 'react';
import { Plus, X, Copy, Check, Pencil } from 'lucide-react';
import { useDesignStore, type Design } from '@/stores/designStore';

// ---------------------------------------------------------------------------
// Single tab
// ---------------------------------------------------------------------------

function DesignTab({
  design,
  isActive,
  canDelete,
}: {
  design: Design;
  isActive: boolean;
  canDelete: boolean;
}) {
  const { setActiveDesign, renameDesign, duplicateDesign, deleteDesign } =
    useDesignStore();
  const [isEditing, setIsEditing] = useState(false);
  const [editName, setEditName] = useState(design.name);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (isEditing) inputRef.current?.focus();
  }, [isEditing]);

  const commitRename = useCallback(() => {
    const trimmed = editName.trim();
    if (trimmed && trimmed !== design.name) {
      renameDesign(design.id, trimmed);
    } else {
      setEditName(design.name);
    }
    setIsEditing(false);
  }, [editName, design.id, design.name, renameDesign]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') commitRename();
    if (e.key === 'Escape') {
      setEditName(design.name);
      setIsEditing(false);
    }
  };

  const hasResult = design.lastResult !== null;

  return (
    <div
      className={`group flex items-center gap-1 px-3 py-1.5 rounded-t-lg border border-b-0 cursor-pointer transition-colors select-none ${
        isActive
          ? 'bg-white border-gray-200 -mb-px z-10'
          : 'bg-gray-50 border-transparent hover:bg-gray-100'
      }`}
      onClick={() => !isEditing && setActiveDesign(design.id)}
    >
      {/* Result dot */}
      {hasResult && (
        <span className="w-1.5 h-1.5 rounded-full bg-green-500 flex-shrink-0" title="Has execution results" />
      )}

      {/* Name */}
      {isEditing ? (
        <input
          ref={inputRef}
          type="text"
          value={editName}
          onChange={(e) => setEditName(e.target.value)}
          onBlur={commitRename}
          onKeyDown={handleKeyDown}
          className="w-24 px-1 py-0 text-xs border border-primary-400 rounded focus:outline-none focus:ring-1 focus:ring-primary-500"
          onClick={(e) => e.stopPropagation()}
        />
      ) : (
        <span
          className={`text-xs font-medium truncate max-w-[120px] ${
            isActive ? 'text-gray-900' : 'text-gray-600'
          }`}
          onDoubleClick={(e) => {
            e.stopPropagation();
            setIsEditing(true);
          }}
        >
          {design.name}
        </span>
      )}

      {/* Actions — visible on hover or when active */}
      <div
        className={`flex items-center gap-0.5 ml-1 ${
          isActive ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'
        } transition-opacity`}
      >
        {!isEditing && (
          <button
            onClick={(e) => {
              e.stopPropagation();
              setIsEditing(true);
            }}
            className="p-0.5 text-gray-400 hover:text-gray-600 rounded"
            title="Rename"
          >
            <Pencil className="w-3 h-3" />
          </button>
        )}
        {isEditing && (
          <button
            onClick={(e) => {
              e.stopPropagation();
              commitRename();
            }}
            className="p-0.5 text-primary-500 hover:text-primary-700 rounded"
            title="Confirm"
          >
            <Check className="w-3 h-3" />
          </button>
        )}
        <button
          onClick={(e) => {
            e.stopPropagation();
            duplicateDesign(design.id);
          }}
          className="p-0.5 text-gray-400 hover:text-gray-600 rounded"
          title="Duplicate"
        >
          <Copy className="w-3 h-3" />
        </button>
        {canDelete && (
          <button
            onClick={(e) => {
              e.stopPropagation();
              deleteDesign(design.id);
            }}
            className="p-0.5 text-gray-400 hover:text-red-500 rounded"
            title="Delete"
          >
            <X className="w-3 h-3" />
          </button>
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Tab bar
// ---------------------------------------------------------------------------

export function DesignTabs() {
  const { designs, activeDesignId, createDesign } = useDesignStore();

  return (
    <div className="flex items-end gap-0.5 px-4 pt-1 bg-gray-100 border-b border-gray-200 overflow-x-auto">
      {designs.map((design) => (
        <DesignTab
          key={design.id}
          design={design}
          isActive={design.id === activeDesignId}
          canDelete={designs.length > 1}
        />
      ))}

      {/* New design button */}
      <button
        onClick={() => createDesign()}
        className="flex items-center gap-1 px-2 py-1.5 text-xs text-gray-500 hover:text-gray-700 hover:bg-gray-200 rounded-t-lg transition-colors"
        title="New design"
      >
        <Plus className="w-3 h-3" />
        New
      </button>
    </div>
  );
}
