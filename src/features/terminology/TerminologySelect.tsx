import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { cn } from "@/lib/utils";

export interface TerminologySelectOption {
  value: string;
  label: string;
}

export function TerminologySelect({
  ariaLabel,
  value,
  options,
  onValueChange,
  disabled,
  className,
}: {
  ariaLabel: string;
  value: string;
  options: TerminologySelectOption[];
  onValueChange: (value: string) => void;
  disabled?: boolean;
  className?: string;
}) {
  return (
    <Select value={value} onValueChange={onValueChange} disabled={disabled}>
      <SelectTrigger
        aria-label={ariaLabel}
        className={cn(
          "h-10 w-full min-w-0 bg-background text-foreground data-[size=default]:h-10 dark:bg-input/30",
          className,
        )}
      >
        <SelectValue />
      </SelectTrigger>
      <SelectContent position="popper" align="start">
        {options.map((option) => (
          <SelectItem
            className="min-h-10 py-2"
            value={option.value}
            key={option.value}
          >
            {option.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
