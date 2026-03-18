import localFont from 'next/font/local';

/**
 * Broadsheet prototype fonts for the editor.
 * Font files symlinked from packages/web-app/public/fonts/woff2/.
 * Only loads weights used in the editor (Regular, Medium, Bold + italics).
 */

export const featureDeck = localFont({
  src: [
    { path: '../public/fonts/woff2/FeatureDeck-Regular.woff2', weight: '400', style: 'normal' },
    { path: '../public/fonts/woff2/FeatureDeck-RegularItalic.woff2', weight: '400', style: 'italic' },
    { path: '../public/fonts/woff2/FeatureDeck-Medium.woff2', weight: '500', style: 'normal' },
    { path: '../public/fonts/woff2/FeatureDeck-MediumItalic.woff2', weight: '500', style: 'italic' },
    { path: '../public/fonts/woff2/FeatureDeck-Bold.woff2', weight: '700', style: 'normal' },
    { path: '../public/fonts/woff2/FeatureDeck-BoldItalic.woff2', weight: '700', style: 'italic' },
  ],
  variable: '--font-feature-deck',
  display: 'swap',
});

export const featureDeckCondensed = localFont({
  src: [
    { path: '../public/fonts/woff2/FeatureDeckCondensed-Regular.woff2', weight: '400', style: 'normal' },
    { path: '../public/fonts/woff2/FeatureDeckCondensed-RegularItalic.woff2', weight: '400', style: 'italic' },
    { path: '../public/fonts/woff2/FeatureDeckCondensed-Medium.woff2', weight: '500', style: 'normal' },
    { path: '../public/fonts/woff2/FeatureDeckCondensed-MediumItalic.woff2', weight: '500', style: 'italic' },
    { path: '../public/fonts/woff2/FeatureDeckCondensed-Bold.woff2', weight: '700', style: 'normal' },
    { path: '../public/fonts/woff2/FeatureDeckCondensed-BoldItalic.woff2', weight: '700', style: 'italic' },
  ],
  variable: '--font-feature-deck-condensed',
  display: 'swap',
});

export const featureText = localFont({
  src: [
    { path: '../public/fonts/woff2/FeatureText-Regular.woff2', weight: '400', style: 'normal' },
    { path: '../public/fonts/woff2/FeatureText-RegularItalic.woff2', weight: '400', style: 'italic' },
    { path: '../public/fonts/woff2/FeatureText-Bold.woff2', weight: '700', style: 'normal' },
    { path: '../public/fonts/woff2/FeatureText-BoldItalic.woff2', weight: '700', style: 'italic' },
  ],
  variable: '--font-feature-text',
  display: 'swap',
});
