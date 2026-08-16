import { notFound } from "next/navigation";

/**
 * Customer BYOK is not part of the billing bridge contract. Keep the historical data and backend
 * code untouched, but make the authenticated product route undiscoverable and unavailable.
 */
export default function ByokPage() {
  notFound();
}
