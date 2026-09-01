import type { ReactNode } from "react";

interface Props {
  title: string;
  onClose: () => void;
  children: ReactNode;
  width?: number;
}

export default function Modal({ title, onClose, children, width = 520 }: Props) {
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-void-0/80 backdrop-blur-sm animate-view-fade"
      onClick={onClose}
    >
      <div
        className="flex max-h-[86vh] w-[--modal-width] max-w-[90vw] flex-col overflow-hidden rounded-2xl border border-white/[0.06] bg-void-4 shadow-2xl shadow-black/70 animate-modal-in"
        style={{ ["--modal-width" as string]: `${width}px` }}
        onClick={(event) => event.stopPropagation()}
      >
        <div className="flex items-center justify-between px-6 pt-6">
          <h2 className="font-display text-xl font-bold text-mist-1">{title}</h2>
          <button
            onClick={onClose}
            className="btn-press flex h-8 w-8 items-center justify-center rounded-full text-mist-3 hover:bg-white/5 hover:text-mist-1"
            aria-label="Fermer"
          >
            ✕
          </button>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto p-6">{children}</div>
      </div>
    </div>
  );
}
