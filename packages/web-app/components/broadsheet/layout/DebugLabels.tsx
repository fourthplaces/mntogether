'use client';

import { useSearchParams } from 'next/navigation';
import { useEffect } from 'react';

export function DebugLabels() {
  const params = useSearchParams();
  const showLabels = params.has('labels');
  const showWeights = params.has('weights');
  useEffect(() => {
    document.body.classList.toggle('show-labels', showLabels);
    document.body.classList.toggle('show-weights', showWeights);
    return () => {
      document.body.classList.remove('show-labels');
      document.body.classList.remove('show-weights');
    };
  }, [showLabels, showWeights]);
  return null;
}
