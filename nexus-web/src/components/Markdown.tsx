// Markdown renderer for assistant messages.
//
// rehype-highlight runs synchronously (lowlight/highlight.js grammars), which
// keeps per-frame re-parse of the streaming source cheap. While streaming, a
// caret character is appended to the markdown source so it sits inline where
// the model's output currently ends.

import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import rehypeHighlight from 'rehype-highlight';
import type { Components } from 'react-markdown';

interface Props {
  text: string;
  streaming?: boolean;
}

const components: Components = {
  a: ({ node: _node, ...props }) => (
    <a
      {...props}
      target="_blank"
      rel="noreferrer"
      className="text-indigo-400 underline decoration-indigo-400/40 underline-offset-2 hover:text-indigo-300"
    />
  ),
  pre: ({ node: _node, ...props }) => (
    <pre
      {...props}
      className="my-2 overflow-x-auto rounded-lg border border-zinc-800 bg-zinc-950 p-3 text-[13px] leading-relaxed"
    />
  ),
  code: ({ node: _node, className, ...props }) => {
    const isBlock = /language-/.test(className ?? '');
    return isBlock ? (
      <code {...props} className={`${className ?? ''} font-mono text-zinc-200`} />
    ) : (
      <code
        {...props}
        className="rounded bg-zinc-800/80 px-1.5 py-0.5 font-mono text-[12px] text-zinc-300"
      />
    );
  },
  table: ({ node: _node, ...props }) => (
    <div className="my-2 overflow-x-auto">
      <table {...props} className="w-full border-collapse text-[13px]" />
    </div>
  ),
  th: ({ node: _node, ...props }) => (
    <th
      {...props}
      className="border border-zinc-700/80 bg-zinc-900 px-2 py-1.5 text-left font-semibold text-zinc-200"
    />
  ),
  td: ({ node: _node, ...props }) => (
    <td {...props} className="border border-zinc-800 px-2 py-1.5 text-zinc-300" />
  ),
  blockquote: ({ node: _node, ...props }) => (
    <blockquote
      {...props}
      className="my-2 border-l-4 border-zinc-700 pl-3 text-zinc-400 italic"
    />
  ),
  img: ({ node: _node, ...props }) => (
    <img {...props} className="my-2 max-w-full rounded-lg border border-zinc-800" />
  ),
  h1: ({ node: _node, ...props }) => (
    <h1 {...props} className="mt-3 mb-2 text-xl font-semibold text-zinc-100" />
  ),
  h2: ({ node: _node, ...props }) => (
    <h2 {...props} className="mt-3 mb-2 text-lg font-semibold text-zinc-100" />
  ),
  h3: ({ node: _node, ...props }) => (
    <h3 {...props} className="mt-2 mb-1.5 text-base font-semibold text-zinc-100" />
  ),
  h4: ({ node: _node, ...props }) => (
    <h4 {...props} className="mt-2 mb-1 text-sm font-semibold text-zinc-100" />
  ),
  ul: ({ node: _node, ...props }) => (
    <ul {...props} className="my-2 list-disc space-y-0.5 pl-5" />
  ),
  ol: ({ node: _node, ...props }) => (
    <ol {...props} className="my-2 list-decimal space-y-0.5 pl-5" />
  ),
  li: ({ node: _node, ...props }) => <li {...props} className="leading-relaxed" />,
  p: ({ node: _node, ...props }) => (
    <p {...props} className="my-1.5 leading-relaxed" />
  ),
  hr: ({ node: _node, ...props }) => (
    <hr {...props} className="my-3 border-zinc-800" />
  ),
};

export default function Markdown({ text, streaming }: Props) {
  const source = streaming ? text + '\u258C' : text;
  return (
    <div className="text-[14px] text-zinc-100">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        rehypePlugins={[rehypeHighlight]}
        components={components}
      >
        {source}
      </ReactMarkdown>
    </div>
  );
}
