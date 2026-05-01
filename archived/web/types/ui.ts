/**
 * Type definitions for Toast Notification and Loading components
 */

// Toast types
export type ToastType = "success" | "error" | "info" | "warning";

export interface Toast {
  id: string;
  type: ToastType;
  message: string;
  duration?: number;
}

export interface ToastContextType {
  toasts: Toast[];
  addToast: (message: string, type?: ToastType, duration?: number) => void;
  removeToast: (id: string) => void;
  success: (message: string, duration?: number) => void;
  error: (message: string, duration?: number) => void;
  info: (message: string, duration?: number) => void;
  warning: (message: string, duration?: number) => void;
}

// Loading component types
export type LoadingSkeletonType = "card" | "list" | "table";
export type LoadingSpinnerSize = "sm" | "md" | "lg";

export interface LoadingSkeletonProps {
  count?: number;
  type?: LoadingSkeletonType;
}

export interface LoadingSpinnerProps {
  size?: LoadingSpinnerSize;
  message?: string;
}

export interface LoadingButtonProps {
  loading?: boolean;
  children: React.ReactNode;
  className?: string;
  onClick?: () => void;
  type?: "button" | "submit" | "reset";
  disabled?: boolean;
}
