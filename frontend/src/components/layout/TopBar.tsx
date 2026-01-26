import { useState } from 'react';
import { Database, Play, GitCompare, Share2, Check, Pencil } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { useCanvasStore } from '@/stores/canvasStore';

export function TopBar() {
  const { designName, setDesignName } = useCanvasStore();
  const [isEditing, setIsEditing] = useState(false);
  const [editedName, setEditedName] = useState(designName);

  const handleNameSubmit = () => {
    if (editedName.trim()) {
      setDesignName(editedName.trim());
    } else {
      setEditedName(designName);
    }
    setIsEditing(false);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      handleNameSubmit();
    } else if (e.key === 'Escape') {
      setEditedName(designName);
      setIsEditing(false);
    }
  };

  return (
    <header className="h-14 bg-white border-b border-gray-200 flex items-center justify-between px-4">
      {/* Logo */}
      <div className="flex items-center gap-3">
        <div className="flex items-center gap-2">
          <div className="w-8 h-8 bg-primary-500 rounded-lg flex items-center justify-center">
            <Database className="w-5 h-5 text-white" />
          </div>
          <span className="font-semibold text-gray-900">DB Simulator</span>
        </div>
      </div>

      {/* Design Name */}
      <div className="flex items-center gap-2">
        {isEditing ? (
          <div className="flex items-center gap-2">
            <input
              type="text"
              value={editedName}
              onChange={(e) => setEditedName(e.target.value)}
              onBlur={handleNameSubmit}
              onKeyDown={handleKeyDown}
              className="px-3 py-1.5 text-sm border border-primary-500 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
              autoFocus
            />
            <Button
              variant="ghost"
              size="sm"
              onClick={handleNameSubmit}
            >
              <Check className="w-4 h-4" />
            </Button>
          </div>
        ) : (
          <button
            onClick={() => setIsEditing(true)}
            className="flex items-center gap-2 px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-100 rounded-lg transition-colors"
          >
            <span>{designName}</span>
            <Pencil className="w-3.5 h-3.5 text-gray-400" />
          </button>
        )}
      </div>

      {/* Actions */}
      <div className="flex items-center gap-2">
        <Button variant="primary" size="sm">
          <Play className="w-4 h-4" />
          Run
        </Button>
        <Button variant="secondary" size="sm">
          <GitCompare className="w-4 h-4" />
          Compare
        </Button>
        <Button variant="ghost" size="sm">
          <Share2 className="w-4 h-4" />
          Share
        </Button>
      </div>
    </header>
  );
}
