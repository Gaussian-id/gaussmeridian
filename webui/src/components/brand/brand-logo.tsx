import { siteConfig } from "@core/config";
import { cn } from "@core/lib/utils";

/** Intrinsic dimensions of the shipped artwork, used to keep every instance aspect-correct. */
const INTRINSIC = {
  lockup: { width: 4412, height: 1264 },
  // The logogram is a square, standalone drawing — not a crop of the lockup.
  mark: { width: 316, height: 316 },
} as const;

type BrandVariant = keyof typeof INTRINSIC;
type BrandTone = "auto" | "light";

interface BrandLogoProps {
  /** `lockup` is the mark plus wordmark; `mark` is the ring alone, for square or narrow slots. */
  variant?: BrandVariant;
  /**
   * `auto` renders both inks and lets the theme class pick one. `light` renders the light ink
   * only, for surfaces whose background is dark regardless of the active theme.
   */
  tone?: BrandTone;
  /** Rendered height in pixels. Width follows from the artwork's aspect ratio. */
  height: number;
  className?: string;
}

/**
 * The single place the GaussMeridian brand becomes an image.
 *
 * Theme handling is deliberately class-driven, not `prefers-color-scheme`. The app runs
 * `next-themes` with `attribute="class"`, so the toggle writes a `dark` class and never touches
 * the OS setting — a `<picture>` media switch would show the wrong ink to anyone who flips it.
 * Both inks are therefore rendered and one is hidden by a `dark:` utility.
 *
 * The accessible name lives on the wrapper as `role="img"` + `aria-label`, not on either image and
 * not as a visually-hidden text node. That announces the brand exactly once whichever ink is
 * visible, and keeps the product name out of the DOM as text — the navbar contract asserts the
 * wordmark is never printed as stale copy beside the logo.
 */
export function BrandLogo({
  variant = "lockup",
  tone = "auto",
  height,
  className,
}: BrandLogoProps) {
  const intrinsic = INTRINSIC[variant];
  const width = Math.round((height * intrinsic.width) / intrinsic.height);

  const image = (ink: "dark" | "light", visibility?: string) => (
    // eslint-disable-next-line @next/next/no-img-element -- static SVG: the image optimiser adds a request and returns no benefit for vectors
    <img
      src={`/logo/meridian-${variant}-${ink}.svg`}
      alt=""
      aria-hidden="true"
      width={width}
      height={height}
      style={{ height, width }}
      className={cn("block max-w-full select-none", visibility, className)}
    />
  );

  return (
    <span
      role="img"
      aria-label={`${siteConfig.name} logo`}
      className="inline-flex shrink-0 items-center"
      style={{ height }}
    >
      {tone === "light" ? (
        image("light")
      ) : (
        <>
          {image("dark", "dark:hidden")}
          {image("light", "hidden dark:block")}
        </>
      )}
    </span>
  );
}

/**
 * The lockup carries two lines of type, so it stops being legible in a narrow header. Below the
 * `sm` breakpoint the mark takes over; from `sm` up the full lockup returns. Only one branch is
 * ever in the accessibility tree, because the other is `display: none`.
 */
export function BrandLogoResponsive({
  tone = "auto",
  markHeight,
  lockupHeight,
  className,
}: {
  tone?: BrandTone;
  markHeight: number;
  lockupHeight: number;
  className?: string;
}) {
  return (
    <>
      <span className="inline-flex sm:hidden">
        <BrandLogo variant="mark" tone={tone} height={markHeight} className={className} />
      </span>
      <span className="hidden sm:inline-flex">
        <BrandLogo variant="lockup" tone={tone} height={lockupHeight} className={className} />
      </span>
    </>
  );
}
