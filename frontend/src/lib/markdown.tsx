// ---------------------------------------------------------------------------
// Shared Markdown renderer — supports headings, lists, code, bold, links
// ---------------------------------------------------------------------------

import React from 'react';

interface MarkdownProps {
  text: string;
  className?: string;
}

type Block =
  | { type: 'heading'; level: 2 | 3; text: string }
  | { type: 'paragraph'; text: string }
  | { type: 'code'; code: string }
  | { type: 'bullet-list'; items: string[] }
  | { type: 'numbered-list'; items: string[] };

function parseBlocks(text: string): Block[] {
  const lines = text.split('\n');
  const blocks: Block[] = [];
  let i = 0;

  while (i < lines.length) {
    const line = lines[i];

    // Skip blank lines
    if (line.trim() === '') {
      i++;
      continue;
    }

    // Code fence
    if (line.trim().startsWith('```')) {
      const codeLines: string[] = [];
      i++;
      while (i < lines.length && !lines[i].trim().startsWith('```')) {
        codeLines.push(lines[i]);
        i++;
      }
      i++; // skip closing ```
      blocks.push({ type: 'code', code: codeLines.join('\n') });
      continue;
    }

    // Heading
    if (line.startsWith('### ')) {
      blocks.push({ type: 'heading', level: 3, text: line.slice(4) });
      i++;
      continue;
    }
    if (line.startsWith('## ')) {
      blocks.push({ type: 'heading', level: 2, text: line.slice(3) });
      i++;
      continue;
    }

    // Bullet list
    if (line.match(/^\s*[-*]\s/)) {
      const items: string[] = [];
      while (i < lines.length && lines[i].match(/^\s*[-*]\s/)) {
        items.push(lines[i].replace(/^\s*[-*]\s+/, ''));
        i++;
      }
      blocks.push({ type: 'bullet-list', items });
      continue;
    }

    // Numbered list
    if (line.match(/^\s*\d+\.\s/)) {
      const items: string[] = [];
      while (i < lines.length && lines[i].match(/^\s*\d+\.\s/)) {
        items.push(lines[i].replace(/^\s*\d+\.\s+/, ''));
        i++;
      }
      blocks.push({ type: 'numbered-list', items });
      continue;
    }

    // Paragraph — collect consecutive non-blank, non-special lines
    const paraLines: string[] = [];
    while (
      i < lines.length &&
      lines[i].trim() !== '' &&
      !lines[i].trim().startsWith('```') &&
      !lines[i].startsWith('## ') &&
      !lines[i].startsWith('### ') &&
      !lines[i].match(/^\s*[-*]\s/) &&
      !lines[i].match(/^\s*\d+\.\s/)
    ) {
      paraLines.push(lines[i]);
      i++;
    }
    if (paraLines.length > 0) {
      blocks.push({ type: 'paragraph', text: paraLines.join(' ') });
    }
  }

  return blocks;
}

function renderBlock(block: Block, key: number): React.ReactNode {
  switch (block.type) {
    case 'heading':
      if (block.level === 2) {
        return (
          <h2 key={key} className="text-base font-semibold text-gray-900 mt-4 mb-1">
            {block.text}
          </h2>
        );
      }
      return (
        <h3 key={key} className="text-sm font-semibold text-gray-800 mt-3 mb-1">
          {block.text}
        </h3>
      );

    case 'code':
      return (
        <pre
          key={key}
          className="bg-gray-100 text-gray-800 rounded-lg px-3 py-2.5 text-xs font-mono overflow-x-auto my-2 border border-gray-200 leading-relaxed whitespace-pre-wrap"
        >
          {block.code}
        </pre>
      );

    case 'bullet-list':
      return (
        <ul key={key} className="my-1.5 space-y-1">
          {block.items.map((item, i) => (
            <li key={i} className="flex gap-2 text-sm text-gray-700">
              <span className="text-gray-400 mt-0.5 shrink-0">•</span>
              <span>{formatInline(item)}</span>
            </li>
          ))}
        </ul>
      );

    case 'numbered-list':
      return (
        <ol key={key} className="my-1.5 space-y-1">
          {block.items.map((item, i) => (
            <li key={i} className="flex gap-2 text-sm text-gray-700">
              <span className="text-gray-500 mt-0 shrink-0 font-mono text-xs w-4 text-right">
                {i + 1}.
              </span>
              <span>{formatInline(item)}</span>
            </li>
          ))}
        </ol>
      );

    case 'paragraph':
      return (
        <p key={key} className="text-sm text-gray-700 leading-relaxed my-1.5">
          {formatInline(block.text)}
        </p>
      );
  }
}

export function formatInline(text: string): React.ReactNode[] {
  const parts: React.ReactNode[] = [];
  const regex = /(\*\*[^*]+\*\*|`[^`]+`|\[[^\]]+\]\([^)]+\))/g;
  let lastIdx = 0;

  text.replace(regex, (match, _p1, offset) => {
    if (offset > lastIdx) {
      parts.push(text.slice(lastIdx, offset));
    }

    if (match.startsWith('**')) {
      parts.push(
        <strong key={offset} className="font-semibold text-gray-900">
          {match.slice(2, -2)}
        </strong>,
      );
    } else if (match.startsWith('`')) {
      parts.push(
        <code
          key={offset}
          className="bg-gray-200 text-gray-800 px-1 py-0.5 rounded text-[11px] font-mono"
        >
          {match.slice(1, -1)}
        </code>,
      );
    } else if (match.startsWith('[')) {
      const linkMatch = match.match(/\[([^\]]+)\]\(([^)]+)\)/);
      if (linkMatch) {
        parts.push(
          <a
            key={offset}
            href={linkMatch[2]}
            target="_blank"
            rel="noopener noreferrer"
            className="text-blue-600 hover:underline"
          >
            {linkMatch[1]}
          </a>,
        );
      }
    }

    lastIdx = offset + match.length;
    return match;
  });

  if (lastIdx < text.length) {
    parts.push(text.slice(lastIdx));
  }

  return parts;
}

export function Markdown({ text, className }: MarkdownProps) {
  const blocks = parseBlocks(text);
  return <div className={className}>{blocks.map((block, i) => renderBlock(block, i))}</div>;
}

/**
 * Lightweight markdown for the AI chat panel (backward compat).
 * Supports: bold, inline code, code blocks, paragraphs.
 */
export function MarkdownLite({ text }: { text: string }) {
  const paragraphs = text.split(/\n\n+/);

  return (
    <>
      {paragraphs.map((para, i) => {
        if (para.startsWith('```')) {
          const code = para.replace(/^```\w*\n?/, '').replace(/```$/, '');
          return (
            <pre
              key={i}
              className="bg-gray-200 text-gray-800 rounded px-2 py-1.5 text-xs font-mono overflow-x-auto my-1.5"
            >
              {code}
            </pre>
          );
        }
        return (
          <p key={i} className={i > 0 ? 'mt-2' : ''}>
            {formatInline(para)}
          </p>
        );
      })}
    </>
  );
}
