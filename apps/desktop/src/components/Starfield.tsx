import { useEffect, useRef } from "react";

interface Star {
  x: number;
  y: number;
  size: number;
  phase: number;
  speed: number;
  depth: number;
}

export default function Starfield({ count = 64 }: { count?: number }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const context = canvas.getContext("2d");
    if (!context) return;

    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    let stars: Star[] = [];
    let width = 0;
    let height = 0;
    let raf = 0;
    let mouseX = 0;
    let mouseY = 0;
    let parallaxX = 0;
    let parallaxY = 0;

    const resize = () => {
      const ratio = Math.min(window.devicePixelRatio || 1, 2);
      const parent = canvas.parentElement;
      width = parent?.clientWidth ?? 0;
      height = parent?.clientHeight ?? 0;
      canvas.width = width * ratio;
      canvas.height = height * ratio;
      canvas.style.width = `${width}px`;
      canvas.style.height = `${height}px`;
      context.setTransform(ratio, 0, 0, ratio, 0, 0);
      stars = Array.from({ length: count }, () => ({
        x: Math.random() * width,
        y: Math.random() * height,
        size: 0.4 + Math.random() * 1.1,
        phase: Math.random() * Math.PI * 2,
        speed: 0.4 + Math.random() * 0.9,
        depth: 0.3 + Math.random() * 0.7,
      }));
    };

    const draw = (time: number) => {
      context.clearRect(0, 0, width, height);
      parallaxX += (mouseX - parallaxX) * 0.03;
      parallaxY += (mouseY - parallaxY) * 0.03;
      for (const star of stars) {
        const twinkle = 0.35 + 0.65 * (0.5 + 0.5 * Math.sin(star.phase + time * 0.001 * star.speed));
        const ox = parallaxX * star.depth * 8;
        const oy = parallaxY * star.depth * 8;
        context.globalAlpha = 0.14 + 0.5 * twinkle * star.depth;
        context.fillStyle = "#cdd3ff";
        context.beginPath();
        context.arc(star.x + ox, star.y + oy, star.size, 0, Math.PI * 2);
        context.fill();
      }
      context.globalAlpha = 1;
    };

    const loop = (time: number) => {
      draw(time);
      raf = requestAnimationFrame(loop);
    };

    const onMouse = (event: MouseEvent) => {
      mouseX = event.clientX / window.innerWidth - 0.5;
      mouseY = event.clientY / window.innerHeight - 0.5;
    };

    resize();
    if (reduced) {
      draw(0);
    } else {
      raf = requestAnimationFrame(loop);
      window.addEventListener("mousemove", onMouse);
    }
    const observer = new ResizeObserver(() => {
      resize();
      if (reduced) draw(0);
    });
    if (canvas.parentElement) observer.observe(canvas.parentElement);

    return () => {
      cancelAnimationFrame(raf);
      window.removeEventListener("mousemove", onMouse);
      observer.disconnect();
    };
  }, [count]);

  return (
    <canvas
      ref={canvasRef}
      className="pointer-events-none absolute inset-0 z-0"
      aria-hidden
    />
  );
}
