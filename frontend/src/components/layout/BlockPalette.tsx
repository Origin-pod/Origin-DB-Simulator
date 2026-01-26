import { useState, type DragEvent } from 'react';
import {
  Search,
  ChevronDown,
  ChevronRight,
  Database,
  Binary,
  HardDrive,
  Layers,
  Cpu,
  Lock,
  GitBranch,
  Archive,
  Grid3x3,
  Sparkles,
  Network,
  Hash,
  FileStack,
  Clock,
  Filter,
  ArrowUpDown,
  Merge,
  FileText,
} from 'lucide-react';
import {
  BLOCK_REGISTRY,
  CATEGORIES,
  getBlocksByCategory,
  searchBlocks,
  type BlockDefinition,
  type CategoryInfo,
} from '@/types/blocks';
import type { BlockCategory } from '@/types';

// Icon mapping
const ICONS: Record<string, React.ReactNode> = {
  Database: <Database className="w-4 h-4" />,
  Binary: <Binary className="w-4 h-4" />,
  HardDrive: <HardDrive className="w-4 h-4" />,
  Layers: <Layers className="w-4 h-4" />,
  Cpu: <Cpu className="w-4 h-4" />,
  Lock: <Lock className="w-4 h-4" />,
  GitBranch: <GitBranch className="w-4 h-4" />,
  Archive: <Archive className="w-4 h-4" />,
  Grid3x3: <Grid3x3 className="w-4 h-4" />,
  Sparkles: <Sparkles className="w-4 h-4" />,
  Network: <Network className="w-4 h-4" />,
  Hash: <Hash className="w-4 h-4" />,
  FileStack: <FileStack className="w-4 h-4" />,
  Clock: <Clock className="w-4 h-4" />,
  Search: <Cpu className="w-4 h-4" />,
  Filter: <Filter className="w-4 h-4" />,
  ArrowUpDown: <ArrowUpDown className="w-4 h-4" />,
  Merge: <Merge className="w-4 h-4" />,
  FileText: <FileText className="w-4 h-4" />,
};

const CATEGORY_STYLES: Record<BlockCategory, { bg: string; border: string; text: string }> = {
  storage: { bg: 'bg-purple-50', border: 'border-storage', text: 'text-storage' },
  index: { bg: 'bg-blue-50', border: 'border-index', text: 'text-index' },
  buffer: { bg: 'bg-teal-50', border: 'border-buffer', text: 'text-buffer' },
  concurrency: { bg: 'bg-amber-50', border: 'border-concurrency', text: 'text-concurrency' },
  execution: { bg: 'bg-pink-50', border: 'border-execution', text: 'text-execution' },
  transaction: { bg: 'bg-indigo-50', border: 'border-transaction', text: 'text-transaction' },
  compression: { bg: 'bg-lime-50', border: 'border-compression', text: 'text-compression' },
  partitioning: { bg: 'bg-orange-50', border: 'border-partitioning', text: 'text-partitioning' },
  optimization: { bg: 'bg-cyan-50', border: 'border-optimization', text: 'text-optimization' },
  distribution: { bg: 'bg-violet-50', border: 'border-distribution', text: 'text-distribution' },
};

interface BlockItemProps {
  block: BlockDefinition;
  onDragStart: (e: DragEvent<HTMLDivElement>, block: BlockDefinition) => void;
}

function BlockItem({ block, onDragStart }: BlockItemProps) {
  const styles = CATEGORY_STYLES[block.category];
  const icon = ICONS[block.icon] || <Database className="w-4 h-4" />;

  return (
    <div
      draggable
      onDragStart={(e) => onDragStart(e, block)}
      className={`
        p-2.5 rounded-lg border-2 cursor-grab active:cursor-grabbing
        ${styles.bg} ${styles.border} border-opacity-50
        hover:border-opacity-100 hover:shadow-sm
        transition-all duration-150
      `}
    >
      <div className="flex items-center gap-2 mb-0.5">
        <span className={styles.text}>{icon}</span>
        <span className="text-sm font-medium text-gray-900 truncate">
          {block.name}
        </span>
      </div>
      <p className="text-xs text-gray-500 line-clamp-2">{block.description}</p>
    </div>
  );
}

interface CategorySectionProps {
  category: CategoryInfo;
  blocks: BlockDefinition[];
  isExpanded: boolean;
  onToggle: () => void;
  onDragStart: (e: DragEvent<HTMLDivElement>, block: BlockDefinition) => void;
}

function CategorySection({
  category,
  blocks,
  isExpanded,
  onToggle,
  onDragStart,
}: CategorySectionProps) {
  const icon = ICONS[category.icon] || <Database className="w-4 h-4" />;

  return (
    <div className="mb-2">
      <button
        onClick={onToggle}
        className="w-full flex items-center gap-2 px-2 py-1.5 text-left hover:bg-gray-50 rounded-lg transition-colors"
      >
        <span className="text-gray-400">
          {isExpanded ? (
            <ChevronDown className="w-4 h-4" />
          ) : (
            <ChevronRight className="w-4 h-4" />
          )}
        </span>
        <span style={{ color: category.color }}>{icon}</span>
        <span className="text-sm font-medium text-gray-700 flex-1">
          {category.name}
        </span>
        <span className="text-xs text-gray-400 bg-gray-100 px-1.5 py-0.5 rounded">
          {blocks.length}
        </span>
      </button>

      {isExpanded && (
        <div className="mt-1 ml-6 space-y-1.5">
          {blocks.map((block) => (
            <BlockItem
              key={block.type}
              block={block}
              onDragStart={onDragStart}
            />
          ))}
        </div>
      )}
    </div>
  );
}

export function BlockPalette() {
  const [searchTerm, setSearchTerm] = useState('');
  const [expandedCategories, setExpandedCategories] = useState<Set<BlockCategory>>(
    new Set(['storage', 'index', 'buffer', 'execution'])
  );

  const handleToggleCategory = (categoryId: BlockCategory) => {
    setExpandedCategories((prev) => {
      const next = new Set(prev);
      if (next.has(categoryId)) {
        next.delete(categoryId);
      } else {
        next.add(categoryId);
      }
      return next;
    });
  };

  const handleDragStart = (e: DragEvent<HTMLDivElement>, block: BlockDefinition) => {
    const dragData = {
      type: block.type,
      name: block.name,
      description: block.description,
      category: block.category,
    };
    e.dataTransfer.setData('application/json', JSON.stringify(dragData));
    e.dataTransfer.effectAllowed = 'move';
  };

  // Get blocks based on search or category
  const isSearching = searchTerm.trim().length > 0;
  const searchResults = isSearching ? searchBlocks(searchTerm) : [];

  // Get active categories (those with blocks)
  const activeCategories = CATEGORIES.filter((cat) =>
    BLOCK_REGISTRY.some((block) => block.category === cat.id)
  );

  return (
    <aside className="w-60 bg-white border-r border-gray-200 flex flex-col">
      {/* Header */}
      <div className="p-3 border-b border-gray-200">
        <h2 className="text-sm font-semibold text-gray-900 mb-2">Blocks</h2>
        <div className="relative">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" />
          <input
            type="text"
            placeholder="Search blocks..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="w-full pl-8 pr-3 py-1.5 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-transparent"
          />
        </div>
      </div>

      {/* Block List */}
      <div className="flex-1 overflow-y-auto p-2">
        {isSearching ? (
          // Search results
          <div className="space-y-1.5">
            {searchResults.length > 0 ? (
              <>
                <p className="text-xs text-gray-500 px-2 py-1">
                  {searchResults.length} result{searchResults.length !== 1 ? 's' : ''}
                </p>
                {searchResults.map((block) => (
                  <BlockItem
                    key={block.type}
                    block={block}
                    onDragStart={handleDragStart}
                  />
                ))}
              </>
            ) : (
              <p className="text-sm text-gray-500 text-center py-8">
                No blocks found for "{searchTerm}"
              </p>
            )}
          </div>
        ) : (
          // Category view
          <div>
            {activeCategories.map((category) => {
              const blocks = getBlocksByCategory(category.id);
              if (blocks.length === 0) return null;

              return (
                <CategorySection
                  key={category.id}
                  category={category}
                  blocks={blocks}
                  isExpanded={expandedCategories.has(category.id)}
                  onToggle={() => handleToggleCategory(category.id)}
                  onDragStart={handleDragStart}
                />
              );
            })}
          </div>
        )}
      </div>

      {/* Footer */}
      <div className="p-2 border-t border-gray-200">
        <p className="text-xs text-gray-400 text-center">
          {BLOCK_REGISTRY.length} blocks available
        </p>
      </div>
    </aside>
  );
}
