import React from 'react';

interface WikiSectionProps {
  id: string;
  title: string;
  icon: React.ReactNode;
  children: React.ReactNode;
}

export function WikiSection({ id, title, icon, children }: WikiSectionProps) {
  return (
    <section id={`wiki-${id}`} className="scroll-mt-4">
      <div className="flex items-center gap-2 mb-2">
        <span className="text-gray-400">{icon}</span>
        <h2 className="text-sm font-semibold text-gray-900">{title}</h2>
      </div>
      <div className="pl-6">{children}</div>
    </section>
  );
}
