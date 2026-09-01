import { useEffect, useState } from "react";
import { getAvatar } from "../lib/void";
import { authorColor } from "../lib/color";

const cache = new Map<string, string | null>();
const pending = new Set<string>();
const listeners = new Set<() => void>();

function notify() {
  listeners.forEach((fn) => fn());
}

// Fetch an avatar once per key. On completion we notify ALL mounted Avatars so
// every instance sharing this key re-reads the cache — not just the one that
// happened to trigger the fetch (that race left duplicates showing the letter).
function loadAvatar(key: string, onionId: string | null) {
  if (cache.has(key) || pending.has(key)) return;
  pending.add(key);
  getAvatar(onionId)
    .then((b64) => {
      const mime = b64.startsWith("/9j/") ? "image/jpeg" : "image/png";
      cache.set(key, b64 ? `data:${mime};base64,${b64}` : null);
    })
    .catch(() => {
      cache.set(key, null);
    })
    .finally(() => {
      pending.delete(key);
      notify();
    });
}

interface Props {
  onionId: string | null;
  name: string;
  size: number;
  className?: string;
}

export default function Avatar({ onionId, name, size, className = "" }: Props) {
  const key = onionId ?? "self";
  const [, setTick] = useState(0);

  useEffect(() => {
    const sync = () => {
      if (!cache.has(key)) loadAvatar(key, onionId);
      setTick((n) => n + 1);
    };
    listeners.add(sync);
    sync();
    return () => {
      listeners.delete(sync);
    };
  }, [key, onionId]);

  const url = cache.get(key) ?? null;

  const style = {
    width: size,
    height: size,
    fontSize: Math.max(10, Math.floor(size * 0.4)),
  };

  if (url) {
    return (
      <img
        src={url}
        alt={name}
        style={style}
        className={`shrink-0 rounded-full object-cover ${className}`}
        onError={() => {
          cache.set(key, null);
          setTick((n) => n + 1);
        }}
      />
    );
  }

  return (
    <div
      style={{ ...style, backgroundColor: onionId ? authorColor(onionId) : "#f5f5f5" }}
      className={`flex shrink-0 items-center justify-center rounded-full font-display font-bold text-void-0 ${className}`}
    >
      {name.slice(0, 1).toUpperCase()}
    </div>
  );
}

export function invalidateAvatarCache(onionId: string | null) {
  if (onionId === null) {
    // Own avatar changed: it is displayed both as "self" (identity preview) and
    // keyed by our own onion id (message bubbles, members column), so drop all.
    cache.clear();
  } else {
    cache.delete(onionId);
  }
  notify();
}
