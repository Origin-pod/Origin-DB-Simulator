import { useEffect, useRef } from 'react';
import { useWikiStore } from '@/stores/wikiStore';
import { getBlockDefinition } from '@/types/blocks';
import { WikiHeader } from './WikiHeader';
import { WikiTOC } from './WikiTOC';
import { WikiContent, buildTOCItems } from './WikiContent';

export function NodeWikiPanel() {
  const { isOpen, blockType, close } = useWikiStore();
  const scrollRef = useRef<HTMLDivElement>(null);

  // Close on Escape
  useEffect(() => {
    if (!isOpen) return;
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') close();
    };
    window.addEventListener('keydown', handleKey);
    return () => window.removeEventListener('keydown', handleKey);
  }, [isOpen, close]);

  // Scroll to top when block changes
  useEffect(() => {
    scrollRef.current?.scrollTo(0, 0);
  }, [blockType]);

  if (!isOpen || !blockType) return null;

  const block = getBlockDefinition(blockType);
  if (!block) return null;

  const tocItems = buildTOCItems(block);

  return (
    <>
      {/* Backdrop */}
      <div
        className="fixed inset-0 bg-black/20 z-40"
        onClick={close}
      />

      {/* Panel */}
      <div className="fixed right-0 top-0 bottom-0 w-[680px] max-w-full bg-white z-50 shadow-2xl flex flex-col">
        <WikiHeader block={block} />

        <div className="flex-1 flex overflow-hidden">
          {/* TOC sidebar */}
          {tocItems.length > 2 && (
            <div className="w-[160px] shrink-0 border-r border-gray-100 py-4 px-3 overflow-y-auto">
              <WikiTOC items={tocItems} scrollContainerRef={scrollRef} />
            </div>
          )}

          {/* Main content */}
          <div ref={scrollRef} className="flex-1 overflow-y-auto px-6 py-5">
            <WikiContent block={block} />
          </div>
        </div>
      </div>
    </>
  );
}
