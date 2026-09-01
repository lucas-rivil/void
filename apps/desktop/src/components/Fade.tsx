import type { ReactNode } from "react";

interface Props {
  id: string;
  children: ReactNode;
}

export default function Fade({ id, children }: Props) {
  return (
    <div key={id} className="flex min-h-0 flex-1 flex-col animate-view-fade">
      {children}
    </div>
  );
}
