// Streaming text renderer. The parent store already updates at display frame
// rate, so this is intentionally dumb — it just renders the accumulated text
// with a blinking caret while streaming.

interface Props {
  text: string;
  streaming?: boolean;
}

export default function StreamingText({ text, streaming }: Props) {
  return (
    <div className="whitespace-pre-wrap break-words leading-relaxed">
      {text}
      {streaming && <span className="caret-blink text-zinc-400">▍</span>}
    </div>
  );
}
