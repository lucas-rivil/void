import { useEffect, useRef, useState } from "react";
import { useI18n } from "../lib/i18n";

interface Props {
  currentColor: string;
  onSave: (color: string) => void;
  onClose: () => void;
}

const HISTORY_KEY = "void-color-history";
const MAX_HISTORY = 8;

function loadHistory(): string[] {
  try {
    return JSON.parse(localStorage.getItem(HISTORY_KEY) ?? "[]");
  } catch {
    return [];
  }
}

function saveHistory(colors: string[]) {
  localStorage.setItem(
    HISTORY_KEY,
    JSON.stringify(colors.slice(0, MAX_HISTORY))
  );
}

function isValidHex(color: string): boolean {
  return /^#[0-9a-fA-F]{6}$/.test(color);
}

export default function ColorPicker({ currentColor, onSave, onClose }: Props) {
  const { t } = useI18n();
  const [color, setColor] = useState(currentColor || "#f5f5f5");
  const [hexInput, setHexInput] = useState(currentColor || "#f5f5f5");
  const [history, setHistory] = useState<string[]>(loadHistory());
  const popupRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const handleClick = (event: MouseEvent) => {
      if (
        popupRef.current &&
        !popupRef.current.contains(event.target as Node)
      ) {
        onClose();
      }
    };
    const handleKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    document.addEventListener("mousedown", handleClick);
    document.addEventListener("keydown", handleKey);
    return () => {
      document.removeEventListener("mousedown", handleClick);
      document.removeEventListener("keydown", handleKey);
    };
  }, [onClose]);

  const updateColor = (newColor: string) => {
    setColor(newColor);
    setHexInput(newColor);
  };

  const handleHexInput = (value: string) => {
    setHexInput(value);
    if (isValidHex(value)) {
      setColor(value);
    }
  };

  const handleSave = () => {
    const next = [color, ...history.filter((c) => c !== color)].slice(
      0,
      MAX_HISTORY
    );
    setHistory(next);
    saveHistory(next);
    onSave(color);
    onClose();
  };

  return (
    <div
      ref={popupRef}
      className="absolute bottom-full left-0 z-50 mb-2 w-[280px] rounded-2xl border border-white/[0.08] bg-void-4 p-4 shadow-2xl shadow-black/70 animate-modal-in"
    >
      <div className="flex items-center gap-3">
        <div className="relative">
          <input
            type="color"
            value={color}
            onChange={(event) => updateColor(event.target.value)}
            className="h-14 w-14 cursor-pointer rounded-xl border border-white/[0.1] bg-transparent"
          />
          <div
            className="pointer-events-none absolute inset-0 rounded-xl border-2 border-white/20"
            style={{ backgroundColor: color + "40" }}
          />
        </div>
        <div className="flex-1">
          <label className="text-[11px] font-bold uppercase tracking-[0.12em] text-mist-3">
            Hex
          </label>
          <input
            value={hexInput}
            onChange={(event) => handleHexInput(event.target.value)}
            maxLength={7}
            spellCheck={false}
            className={`focus-glow mt-1 w-full rounded-xl border px-3 py-2 font-mono text-[14px] uppercase outline-none ${
              isValidHex(hexInput)
                ? "border-white/[0.07] bg-void-2 text-mist-1"
                : "border-alerte/40 bg-void-2 text-alerte"
            }`}
            placeholder="#RRGGBB"
          />
        </div>
      </div>

      {history.length > 0 && (
        <div className="mt-3">
          <p className="text-[11px] font-bold uppercase tracking-[0.12em] text-mist-3">
            {t("colorPicker.history")}
          </p>
          <div className="mt-1.5 flex flex-wrap gap-1.5">
            {history.map((c) => (
              <button
                key={c}
                onClick={() => updateColor(c)}
                className={`btn-press h-7 w-7 rounded-lg border transition-transform hover:scale-110 ${
                  color === c ? "border-white" : "border-white/15"
                }`}
                style={{ backgroundColor: c }}
                aria-label={c}
              />
            ))}
          </div>
        </div>
      )}

      <div className="mt-4 flex items-center gap-2">
        <div
          className="h-9 flex-1 rounded-xl border border-white/[0.06]"
          style={{ backgroundColor: color }}
        />
        <button
          onClick={onClose}
          className="btn-press rounded-xl bg-void-2 px-3.5 py-2 text-[13px] font-medium text-mist-2 hover:text-mist-1"
        >
          {t("common.cancel")}
        </button>
        <button
          onClick={handleSave}
          disabled={!isValidHex(hexInput)}
          className="btn-press focus-glow rounded-xl bg-nebula px-4 py-2 font-display text-[12px] font-bold uppercase tracking-wider text-void-0 shadow-lg shadow-nebula/25 hover:bg-nebula-hi disabled:opacity-50 disabled:shadow-none"
        >
          {t("identity.save")}
        </button>
      </div>
    </div>
  );
}
