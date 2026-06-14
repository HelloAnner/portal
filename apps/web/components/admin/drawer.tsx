"use client";

import * as React from "react";
import * as Dialog from "@radix-ui/react-dialog";
import { X } from "lucide-react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";

interface DrawerProps {
  open: boolean;
  onClose: () => void;
  title: React.ReactNode;
  children: React.ReactNode;
  footer?: React.ReactNode;
  width?: string;
}

export function Drawer({
  open,
  onClose,
  title,
  children,
  footer,
  width = "480px",
}: DrawerProps) {
  return (
    <Dialog.Root open={open} onOpenChange={(v) => !v && onClose()}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 z-40 bg-black/20" />
        <Dialog.Content
          style={{ width }}
          className={cn(
            "fixed inset-y-0 right-0 z-50 flex flex-col border-l border-border-subtle bg-bg-secondary shadow-xl"
          )}
        >
          <div className="flex h-[60px] items-center justify-between border-b border-border-subtle px-5">
            <Dialog.Title className="text-base font-semibold text-text-primary">
              {title}
            </Dialog.Title>
            <Button variant="ghost" onClick={onClose} className="h-8 w-8 p-0" aria-label="关闭">
              <X className="h-4 w-4" />
            </Button>
          </div>
          <div className="flex-1 overflow-auto p-5">{children}</div>
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
