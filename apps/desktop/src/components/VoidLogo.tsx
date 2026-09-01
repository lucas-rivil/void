interface Props {
  size?: number;
  className?: string;
  orbit?: boolean;
}

export default function VoidLogo({ size = 48, className = "", orbit = true }: Props) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 48 48"
      className={className}
      aria-label="Void"
    >
      <circle
        cx="24"
        cy="24"
        r="21"
        fill="none"
        stroke="currentColor"
        strokeOpacity="0.28"
        strokeWidth="1.2"
      />
      <circle cx="24" cy="24" r="13.5" fill="#05060a" />
      <circle
        cx="24"
        cy="24"
        r="13.5"
        fill="none"
        stroke="currentColor"
        strokeOpacity="0.75"
        strokeWidth="1.4"
      />
      <text
        x="24"
        y="30.5"
        textAnchor="middle"
        fontFamily='"Space Grotesk", sans-serif'
        fontWeight="700"
        fontSize="15"
        fill="currentColor"
      >
        V
      </text>
      {orbit && (
        <g
          className="animate-spin-slow"
          style={{ transformOrigin: "24px 24px" }}
        >
          <circle cx="24" cy="3" r="1.9" fill="currentColor" />
          <circle
            cx="45"
            cy="24"
            r="1.1"
            fill="currentColor"
            opacity="0.5"
          />
        </g>
      )}
    </svg>
  );
}
