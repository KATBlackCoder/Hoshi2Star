import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

/** Message of a thrown value: an `Error`'s message, or the value as text. */
export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}
