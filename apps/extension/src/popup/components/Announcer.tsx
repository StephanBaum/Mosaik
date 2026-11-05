/**
 * The popup's live region: "Copied. Clears in 30 seconds.", "Filled.", the outcome of a save.
 * The text is visible too, in a bar between the page and the footer (rendered by the shell, in
 * the layout flow so it covers nothing), and clears itself after a few seconds.
 * Everything that happens without a page change is said here, not only drawn.
 */
import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";

const SHOWN_MS = 6000;

type Announce = (message: string) => void;

const AnnounceContext = createContext<Announce | null>(null);
const MessageContext = createContext<{ text: string; count: number }>({ text: "", count: 0 });

export function AnnouncerProvider({ children }: { children: ReactNode }) {
  const [message, setMessage] = useState({ text: "", count: 0 });
  const announce = useCallback<Announce>((text) => {
    setMessage((previous) => ({ text, count: previous.count + 1 }));
  }, []);
  useEffect(() => {
    if (message.text === "") {
      return undefined;
    }
    const timer = setTimeout(() => {
      setMessage((previous) => ({ ...previous, text: "" }));
    }, SHOWN_MS);
    return () => {
      clearTimeout(timer);
    };
  }, [message]);
  const value = useMemo(() => announce, [announce]);
  return (
    <AnnounceContext value={value}>
      <MessageContext value={message}>{children}</MessageContext>
    </AnnounceContext>
  );
}

/** The visible, polite live region. The shell puts it between the page and the footer. */
export function StatusBar() {
  const message = useContext(MessageContext);
  return (
    <div className="status-bar" role="status" aria-live="polite" aria-atomic="true">
      {/* A new node per announcement, so the same words twice in a row are read twice. */}
      {message.text === "" ? null : <span key={message.count}>{message.text}</span>}
    </div>
  );
}

export function useAnnounce(): Announce {
  const announce = useContext(AnnounceContext);
  if (announce === null) {
    throw new Error("useAnnounce needs an AnnouncerProvider");
  }
  return announce;
}
