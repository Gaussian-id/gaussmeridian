/** The Meridian brand mark: a minimal wireframe meridian globe (one lit longitude = "the line"). */
export function MeridianMark({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      aria-hidden="true"
      className={className}
    >
      <circle cx="12" cy="12" r="9" />
      <ellipse cx="12" cy="12" rx="3.6" ry="9" />
      <path d="M3.2 12h17.6" />
    </svg>
  );
}
