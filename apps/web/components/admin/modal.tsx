"use client";

import * as React from "react";
import * as Dialog from "@radix-ui/react-dialog";
import { X } from "lucide-react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";

interface ModalProps {
  open: boolean;
  onClose: () => void;
  title: React.ReactNode;
  children: React.ReactNode;
  footer?: React.ReactNode;
  maxWidth?: string;
}

export function Modal({
  open,
  onClose,
  title,
  children,
  footer,
  maxWidth = "560px",
}: ModalProps) {
  return (
    <Dialog.Root open={open} onOpenChange={(v) => !v && onClose()}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 z-40 bg-black/20" />
        <Dialog.Content
          style={{ maxWidth }}
          className={cn(
            "fixed left-1/2 top-1/2 z-50 w-[calc(100%-2rem)] -translate-x-1/2 -translate-y-1/2 rounded-radius-lg border border-border-subtle bg-bg-secondary p-0 shadow-xl"
          )}
        >
          <div className="flex items-center justify-between border-b border-border-subtle px-5 py-4">
            <Dialog.Title className="text-base font-semibold text-text-primary">
              {title}
            </Dialog.Title>
            <Button variant="ghost" onClick={onClose} className="h-8 w-8 p-0" aria-label="关闭">
              <X className="h-4 w-4" />
            </Button>
          </div>
          <div className="max-h-[calc(100vh-180px)] overflow-auto px-5 py-5">
            {children}
          </div>
          {footer && (
            <div className="flex items-center justify-end gap-3 border-t border-border-subtle px-5 py-4">
              {footer}
            </div>
          )}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
