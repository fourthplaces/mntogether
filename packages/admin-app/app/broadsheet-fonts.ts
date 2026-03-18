import localFont from 'next/font/local';

/**
 * Broadsheet prototype fonts for the editor.
 * Font files symlinked from packages/web-app/public/fonts/.
 * Only loads weights used in the editor (Regular, Medium, Bold + italics).
 */

export const featureDeck = localFont({
  src: [
    { path: '../public/fonts/FeatureDeck-Regular.woff2', weight: '400', style: 'normal' },
    { path: '../public/fonts/FeatureDeck-RegularItalic.woff2', weight: '400', style: 'italic' },
    { path: '../public/fonts/FeatureDeck-Medium.woff2', weight: '500', style: 'normal' },
    { path: '../public/fonts/FeatureDeck-MediumItalic.woff2', weight: '500', style: 'italic' },
    { path: '../public/fonts/FeatureDeck-Bold.woff2', weight: '700', style: 'normal' },
    { path: '../public/fonts/FeatureDeck-BoldItalic.woff2', weight: '700', style: 'italic' },
  ],
  variable: '--font-feature-deck',
  display: 'swap',
});

export const featureDeckCondensed = localFont({
  src: [
    { path: '../public/fonts/FeatureDeckCondensed-Regular.woff2', weight: '400', style: 'normal' },
    { path: '../public/fonts/FeatureDeckCondensed-RegularItalic.woff2', weight: '400', style: 'italic' },
    { path: '../public/fonts/FeatureDeckCondensed-Medium.woff2', weight: '500', style: 'normal' },
    { path: '../public/fonts/FeatureDeckCondensed-MediumItalic.woff2', weight: '500', style: 'italic' },
    { path: '../public/fonts/FeatureDeckCondensed-Bold.woff2', weight: '700', style: 'normal' },
    { path: '../public/fonts/FeatureDeckCondensed-BoldItalic.woff2', weight: '700', style: 'italic' },
  ],
  variable: '--font-feature-deck-condensed',
  display: 'swap',
});

export const featureText = localFont({
  src: [
    { path: '../public/fonts/FeatureText-Regular.woff2', weight: '400', style: 'normal' },
    { path: '../public/fonts/FeatureText-RegularItalic.woff2', weight: '400', style: 'italic' },
    { path: '../public/fonts/FeatureText-Bold.woff2', weight: '700', style: 'normal' },
    { path: '../public/fonts/FeatureText-BoldItalic.woff2', weight: '700', style: 'italic' },
  ],
  variable: '--font-feature-text',
  display: 'swap',
});
