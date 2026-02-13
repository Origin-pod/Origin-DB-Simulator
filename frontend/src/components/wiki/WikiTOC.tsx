import { useState, useEffect } from 'react';

export interface TOCItem {
  id: string;
  label: string;
}

interface WikiTOCProps {
  items: TOCItem[];
  scrollContainerRef: React.RefObject<HTMLDivElement | null>;
}

export function WikiTOC({ items, scrollContainerRef }: WikiTOCProps) {
  const [activeId, setActiveId] = useState(items[0]?.id ?? '');

  useEffect(() => {
    const container = scrollContainerRef.current;
    if (!container) return;

    const observer = new IntersectionObserver(
      (entries) => {
        // Pick the first intersecting section
        for (const entry of entries) {
          if (entry.isIntersecting) {
            const sectionId = entry.target.id.replace('wiki-', '');
            setActiveId(sectionId);
            break;
          }
        }
      },
      {
        root: container,
        rootMargin: '-10% 0px -80% 0px',
        threshold: 0,
      },
    );

    for (const item of items) {
      const el = container.querySelector(`#wiki-${item.id}`);
      if (el) observer.observe(el);
    }

    return () => observer.disconnect();
  }, [items, scrollContainerRef]);

  const scrollTo = (id: string) => {
    const el = scrollContainerRef.current?.querySelector(`#wiki-${id}`);
    el?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  };

  return (
    <nav className="space-y-0.5">
      {items.map((item) => (
        <button
          key={item.id}
          onClick={() => scrollTo(item.id)}
          className={`block w-full text-left text-xs px-2 py-1 rounded transition-colors ${
            activeId === item.id
              ? 'text-blue-700 bg-blue-50 font-medium'
              : 'text-gray-500 hover:text-gray-700 hover:bg-gray-50'
          }`}
        >
          {item.label}
        </button>
      ))}
    </nav>
  );
}
