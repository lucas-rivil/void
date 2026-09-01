import { useEffect, useState, useCallback } from "react";
import { readText, writeText } from "@tauri-apps/plugin-clipboard-manager";
import { useI18n } from "./i18n";

export interface MenuItem {
  label?: string;
  icon?: string;
  action?: () => void;
  separator?: boolean;
  danger?: boolean;
}

type Editable = HTMLInputElement | HTMLTextAreaElement;

function isEditable(el: EventTarget | null): el is Editable {
  if (!(el instanceof HTMLElement)) return false;
  if (el instanceof HTMLTextAreaElement) return !el.disabled && !el.readOnly;
  if (el instanceof HTMLInputElement) {
    const textLike = [
      "text",
      "search",
      "url",
      "tel",
      "password",
      "email",
      "number",
      "",
    ].includes(el.type);
    return textLike && !el.disabled && !el.readOnly;
  }
  return false;
}

// Controlled React inputs track their value internally; the native setter +
// input event is the supported way to mutate one from outside React.
function setNativeValue(el: Editable, value: string) {
  const proto =
    el instanceof HTMLTextAreaElement
      ? HTMLTextAreaElement.prototype
      : HTMLInputElement.prototype;
  const setter = Object.getOwnPropertyDescriptor(proto, "value")?.set;
  setter?.call(el, value);
  el.dispatchEvent(new Event("input", { bubbles: true }));
}

function selectionOf(el: Editable): [number, number] {
  return [el.selectionStart ?? 0, el.selectionEnd ?? el.value.length];
}

async function copyFromEditable(el: Editable) {
  const [start, end] = selectionOf(el);
  const text = start === end ? el.value : el.value.slice(start, end);
  if (text) await writeText(text);
}

async function cutFromEditable(el: Editable) {
  const [start, end] = selectionOf(el);
  if (start === end) return;
  await writeText(el.value.slice(start, end));
  setNativeValue(el, el.value.slice(0, start) + el.value.slice(end));
  el.focus();
  el.setSelectionRange(start, start);
}

async function pasteIntoEditable(el: Editable) {
  const clip = await readText();
  if (!clip) return;
  const [start, end] = selectionOf(el);
  const next = el.value.slice(0, start) + clip + el.value.slice(end);
  setNativeValue(el, next);
  const caret = start + clip.length;
  el.focus();
  el.setSelectionRange(caret, caret);
}

function ContextMenuDropdown({
  menu,
  close,
}: {
  menu: { x: number; y: number; items: MenuItem[] };
  close: () => void;
}) {
  const maxX = window.innerWidth - 220;
  const maxY = window.innerHeight - (menu.items.length * 40 + 16);
  const x = Math.min(menu.x, Math.max(8, maxX));
  const y = Math.min(menu.y, Math.max(8, maxY));

  return (
    <div
      className="fixed z-[999] min-w-[200px] overflow-hidden rounded-xl border border-white/[0.08] bg-void-4 py-1.5 shadow-2xl shadow-black/80 animate-modal-in"
      style={{ left: x, top: y }}
      onContextMenu={(e) => e.preventDefault()}
    >
      {menu.items.map((item, index) =>
        item.separator ? (
          <div key={index} className="my-1 h-px bg-white/[0.06]" />
        ) : (
          <button
            key={index}
            onClick={() => {
              item.action?.();
              close();
            }}
            className={`flex w-full items-center gap-2.5 px-4 py-2 text-left text-[14px] transition-colors ${
              item.danger
                ? "text-alerte hover:bg-alerte/10"
                : "text-mist-1 hover:bg-white/[0.06]"
            }`}
          >
            {item.icon && (
              <span className="w-4 text-center text-[13px] text-mist-3">
                {item.icon}
              </span>
            )}
            <span className="flex-1">{item.label}</span>
          </button>
        )
      )}
    </div>
  );
}

// A single app-wide styled context menu that replaces the native one. It offers
// cut/copy/paste/select-all in text fields, copy on a selection, and copy-link
// on anchors. Right-clicking anywhere else simply suppresses the native menu.
export function useGlobalContextMenu() {
  const { t } = useI18n();
  const [menu, setMenu] = useState<{
    x: number;
    y: number;
    items: MenuItem[];
  } | null>(null);

  const close = useCallback(() => setMenu(null), []);

  useEffect(() => {
    const onContextMenu = (event: MouseEvent) => {
      event.preventDefault();
      const target = event.target;
      const items: MenuItem[] = [];
      const selection = window.getSelection()?.toString() ?? "";
      const anchor =
        target instanceof Element
          ? (target.closest("a[href]") as HTMLAnchorElement | null)
          : null;

      if (isEditable(target)) {
        const el = target;
        const [start, end] = selectionOf(el);
        const hasSelection = start !== end;
        if (hasSelection) {
          items.push({
            label: t("ctx.cut"),
            icon: "✂",
            action: () => void cutFromEditable(el),
          });
          items.push({
            label: t("ctx.copy"),
            icon: "⧉",
            action: () => void copyFromEditable(el),
          });
        }
        items.push({
          label: t("ctx.paste"),
          icon: "⎘",
          action: () => void pasteIntoEditable(el),
        });
        if (el.value.length > 0) {
          items.push({ separator: true });
          items.push({
            label: t("ctx.selectAll"),
            action: () => {
              el.focus();
              el.select();
            },
          });
        }
      } else if (anchor) {
        items.push({
          label: t("ctx.copyLink"),
          icon: "⧉",
          action: () => void writeText(anchor.href),
        });
        if (selection) {
          items.push({
            label: t("ctx.copy"),
            action: () => void writeText(selection),
          });
        }
      } else if (selection) {
        items.push({
          label: t("ctx.copy"),
          icon: "⧉",
          action: () => void writeText(selection),
        });
      }

      if (items.length === 0) {
        setMenu(null);
        return;
      }
      setMenu({ x: event.clientX, y: event.clientY, items });
    };

    document.addEventListener("contextmenu", onContextMenu);
    return () => document.removeEventListener("contextmenu", onContextMenu);
  }, [t]);

  useEffect(() => {
    if (!menu) return;
    const onClose = () => close();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    window.addEventListener("click", onClose);
    window.addEventListener("keydown", onKey);
    window.addEventListener("blur", onClose);
    window.addEventListener("resize", onClose);
    window.addEventListener("scroll", onClose, true);
    return () => {
      window.removeEventListener("click", onClose);
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("blur", onClose);
      window.removeEventListener("resize", onClose);
      window.removeEventListener("scroll", onClose, true);
    };
  }, [menu, close]);

  return menu ? <ContextMenuDropdown menu={menu} close={close} /> : null;
}

// Suppresses browser-only shortcuts and image/link dragging inside the app.
// The native context menu is handled by useGlobalContextMenu; browser
// accelerator keys (Ctrl+F, F5, devtools…) are disabled at the WebView level.
export function useDisableBrowserFeatures() {
  useEffect(() => {
    const blocked = new Set(["F3", "F7", "F12"]);
    const blockedCombos = new Set([
      "ctrl+f",
      "ctrl+p",
      "ctrl+s",
      "ctrl+g",
      "ctrl+h",
      "ctrl+j",
      "ctrl+u",
      "ctrl+shift+i",
      "ctrl+shift+j",
      "ctrl+shift+c",
      "ctrl+r",
      "ctrl+shift+r",
      "ctrl+d",
      "ctrl+l",
    ]);

    const handleKeyDown = (event: KeyboardEvent) => {
      const key = event.key.toLowerCase();
      const combo = `${event.ctrlKey ? "ctrl+" : ""}${
        event.shiftKey ? "shift+" : ""
      }${key}`;
      if (blockedCombos.has(combo) || blocked.has(event.key)) {
        event.preventDefault();
        event.stopPropagation();
      }
    };

    const handleDragStart = (event: DragEvent) => {
      if (event.target instanceof HTMLElement) {
        const tag = event.target.tagName.toLowerCase();
        if (tag === "img" || tag === "a") {
          event.preventDefault();
        }
      }
    };

    document.addEventListener("keydown", handleKeyDown, true);
    document.addEventListener("dragstart", handleDragStart);

    return () => {
      document.removeEventListener("keydown", handleKeyDown, true);
      document.removeEventListener("dragstart", handleDragStart);
    };
  }, []);
}
