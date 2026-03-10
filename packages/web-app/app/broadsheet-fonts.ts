import localFont from 'next/font/local';
import { GeistMono } from 'geist/font/mono';

export const featureDeck = localFont({
  src: [
    { path: '../public/fonts/woff2/FeatureDeck-Light.woff2', weight: '300', style: 'normal' },
    { path: '../public/fonts/woff2/FeatureDeck-LightItalic.woff2', weight: '300', style: 'italic' },
    { path: '../public/fonts/woff2/FeatureDeck-Regular.woff2', weight: '400', style: 'normal' },
    { path: '../public/fonts/woff2/FeatureDeck-RegularItalic.woff2', weight: '400', style: 'italic' },
    { path: '../public/fonts/woff2/FeatureDeck-Medium.woff2', weight: '500', style: 'normal' },
    { path: '../public/fonts/woff2/FeatureDeck-MediumItalic.woff2', weight: '500', style: 'italic' },
    { path: '../public/fonts/woff2/FeatureDeck-Bold.woff2', weight: '700', style: 'normal' },
    { path: '../public/fonts/woff2/FeatureDeck-BoldItalic.woff2', weight: '700', style: 'italic' },
    { path: '../public/fonts/woff2/FeatureDeck-ExtraBold.woff2', weight: '800', style: 'normal' },
    { path: '../public/fonts/woff2/FeatureDeck-ExtraBoldItalic.woff2', weight: '800', style: 'italic' },
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
    { path: '../public/fonts/woff2/FeatureDeckCondensed-ExtraBold.woff2', weight: '800', style: 'normal' },
    { path: '../public/fonts/woff2/FeatureDeckCondensed-ExtraBoldItalic.woff2', weight: '800', style: 'italic' },
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
    { path: '../public/fonts/woff2/FeatureText-ExtraBold.woff2', weight: '800', style: 'normal' },
    { path: '../public/fonts/woff2/FeatureText-ExtraBoldItalic.woff2', weight: '800', style: 'italic' },
  ],
  variable: '--font-feature-text',
  display: 'swap',
});

export const blissfulRadiance = localFont({
  src: [
    { path: '../public/fonts/woff2/BlissfulRadiance-Regular.woff', weight: '400', style: 'normal' },
  ],
  variable: '--font-blissful-radiance',
  display: 'block',
});

export const geistMono = GeistMono;
