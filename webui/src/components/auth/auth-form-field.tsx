import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

interface AuthFormFieldProps {
  id: string;
  label: string;
  type?: string;
  autoComplete?: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  minLength?: number;
  /**
   * Field-level error (e.g. "That username is taken"). When set, marks the input
   * `aria-invalid` and wires `aria-describedby` to the message for screen readers.
   */
  error?: string;
}

/** Labeled input field shared by every (auth) form — login, signup, forgot-password. */
export function AuthFormField({
  id,
  label,
  type = "text",
  autoComplete,
  value,
  onChange,
  placeholder,
  minLength,
  error,
}: AuthFormFieldProps) {
  const errorId = `${id}-error`;
  return (
    <div className="flex flex-col gap-1.5">
      <Label htmlFor={id}>{label}</Label>
      <Input
        id={id}
        type={type}
        autoComplete={autoComplete}
        required
        minLength={minLength}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={placeholder}
        aria-invalid={error ? true : undefined}
        aria-describedby={error ? errorId : undefined}
      />
      {error && (
        <p id={errorId} className="text-destructive text-sm">
          {error}
        </p>
      )}
    </div>
  );
}
