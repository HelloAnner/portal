"use client";

import * as React from "react";

interface CodeBlockProps {
  code: string;
  language?: string;
}

export function CodeBlock({ code, language }: CodeBlockProps) {
  return (
    <div className="rounded-radius-sm border border-border-subtle bg-bg-tertiary p-4">
      {language && <div className="mb-2 text-xs text-text-muted">{language}</div>}
      <pre className="overflow-auto text-xs text-text-secondary font-mono whitespace-pre">{code}</pre>
    </div>
  );
}
