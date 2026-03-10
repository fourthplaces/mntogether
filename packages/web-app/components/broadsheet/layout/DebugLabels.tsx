'use client';

import { useSearchParams } from 'next/navigation';
import { useEffect } from 'react';

export function DebugLabels() {
  const params = useSearchParams();
  const active = params.has('labels');
  useEffect(() => {
    document.body.classList.toggle('show-labels', active);
    return () => document.body.classList.remove('show-labels');
  }, [active]);
  return null;
}
