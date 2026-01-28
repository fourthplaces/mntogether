# Designer Guide: Minnesota Digital Aid

This guide helps designers customize the visual appearance of the Minnesota Digital Aid platform.

## Project Overview

Minnesota Digital Aid is a volunteer matching platform with two frontend applications:

1. **Admin Web App** (`packages/admin-spa`) - React web application for administrators
2. **Mobile App** (`packages/app`) - React Native mobile app for volunteers

## Quick Start for Designers

### Admin Web App (Tailwind CSS)
- **Technology**: React + Vite + Tailwind CSS
- **Styling Method**: Utility-first CSS classes
- **Main Files**:
  - `packages/admin-spa/src/index.css` - Global styles and base configuration
  - `packages/admin-spa/tailwind.config.js` - Theme customization (colors, spacing, fonts)
  - `packages/admin-spa/src/**/*.tsx` - Component files with inline Tailwind classes

### Mobile App (React Native)
- **Technology**: React Native + Expo
- **Styling Method**: StyleSheet API (inline styles)
- **Main Files**:
  - `packages/app/src/screens/*.tsx` - Screen components with StyleSheet definitions
  - Each component defines its own styles at the bottom of the file

## Color Palette

### Current Colors

The application uses a consistent color scheme across both platforms:

#### Primary Colors
- **Primary Blue**: `#2563eb` (rgb(37, 99, 235))
  - Used for: Primary buttons, links, active states
  - Hover: `#1d4ed8`

#### Semantic Colors
- **Success Green**: `#16a34a` (rgb(22, 163, 74))
  - Used for: Approve buttons, success messages
  - Hover: `#15803d`

- **Danger Red**: `#dc2626` (rgb(220, 38, 38))
  - Used for: Reject buttons, error messages
  - Hover: `#b91c1c`

- **Warning Yellow/Amber**: `#fef3c7` (rgb(254, 243, 199))
  - Used for: Warning badges, medium urgency indicators

#### Neutral Colors
- **Gray Scale**:
  - Background: `#f3f4f6` (gray-100)
  - Border: `#e5e7eb` (gray-200)
  - Text Secondary: `#6b7280` (gray-500)
  - Text Primary: `#111827` (gray-900)
  - Text Body: `#374151` (gray-700)
  - Muted Text: `#9ca3af` (gray-400)

- **White**: `#ffffff`
  - Used for: Card backgrounds, button text

### Urgency Indicators
- **Urgent**: `#fee2e2` (red-100 background), `#92400e` (text)
- **Medium**: `#fef3c7` (yellow-100 background), `#92400e` (text)
- **Low**: `#dbeafe` (blue-100 background), `#1e40af` (text)

## Typography

### Font Families
- **Primary Font**: `Inter, system-ui, Avenir, Helvetica, Arial, sans-serif`
- **Fallback**: System default fonts for mobile

### Font Sizes (Admin Web App - Tailwind)
- **Heading 1**: `text-3xl` (1.875rem / 30px)
- **Heading 2**: `text-2xl` (1.5rem / 24px)
- **Heading 3**: `text-xl` (1.25rem / 20px)
- **Body Large**: `text-lg` (1.125rem / 18px)
- **Body**: `text-base` (1rem / 16px)
- **Small**: `text-sm` (0.875rem / 14px)
- **Extra Small**: `text-xs` (0.75rem / 12px)

### Font Sizes (Mobile App - StyleSheet)
- **Large Title**: `28px` (weight: 700)
- **Title**: `18-20px` (weight: 600)
- **Body**: `16px` (weight: 400-500)
- **Secondary**: `14px` (weight: 400-500)
- **Small**: `12-14px` (weight: 600)

### Font Weights
- **Bold**: `700` or `font-bold`
- **Semibold**: `600` or `font-semibold`
- **Medium**: `500` or `font-medium`
- **Regular**: `400` or `font-normal`

## Spacing System

Both apps use consistent spacing based on multiples of 4px:

- **2**: 8px
- **3**: 12px
- **4**: 16px
- **6**: 24px
- **8**: 32px
- **12**: 48px
- **16**: 64px

## Component Patterns

### Buttons

#### Primary Button (Admin Web)
```html
<button className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700">
  Button Text
</button>
```

#### Success Button (Admin Web)
```html
<button className="px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700">
  ✓ Approve
</button>
```

#### Danger Button (Admin Web)
```html
<button className="px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700">
  ✗ Reject
</button>
```

#### Primary Button (Mobile)
```javascript
{
  backgroundColor: '#2563eb',
  padding: 16,
  borderRadius: 12,
  alignItems: 'center',
}
```

### Cards

#### Admin Web Card
```html
<div className="bg-white border border-gray-200 rounded-lg p-6 hover:shadow-lg transition-shadow">
  Card Content
</div>
```

#### Mobile Card
```javascript
{
  backgroundColor: 'white',
  borderRadius: 12,
  padding: 16,
  marginBottom: 16,
  shadowColor: '#000',
  shadowOffset: { width: 0, height: 2 },
  shadowOpacity: 0.1,
  shadowRadius: 4,
  elevation: 3,
}
```

### Badges

#### Admin Web Badge
```html
<span className="text-xs font-medium px-2 py-1 bg-gray-100 rounded">
  Badge Text
</span>
```

#### Mobile Badge
```javascript
{
  paddingHorizontal: 8,
  paddingVertical: 4,
  borderRadius: 12,
  backgroundColor: '#fef3c7',
}
```

### Modals (Admin Web)
```html
<div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4">
  <div className="bg-white rounded-lg max-w-2xl w-full max-h-[80vh] overflow-y-auto p-6">
    Modal Content
  </div>
</div>
```

## Customization Guide

### Admin Web App (Tailwind)

#### Method 1: Extend Theme in tailwind.config.js
This is the **recommended approach** for global design changes.

```javascript
// packages/admin-spa/tailwind.config.js
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        primary: {
          50: '#eff6ff',
          100: '#dbeafe',
          200: '#bfdbfe',
          300: '#93c5fd',
          400: '#60a5fa',
          500: '#3b82f6',  // Your primary blue
          600: '#2563eb',  // Default primary
          700: '#1d4ed8',
          800: '#1e40af',
          900: '#1e3a8a',
        },
        success: {
          600: '#16a34a',
          700: '#15803d',
        },
        danger: {
          600: '#dc2626',
          700: '#b91c1c',
        }
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
      },
      borderRadius: {
        'card': '12px',
        'button': '8px',
      },
      spacing: {
        'card': '24px',
      }
    },
  },
  plugins: [],
}
```

After updating the config, you can use custom values:
```html
<button className="bg-primary-600 hover:bg-primary-700 rounded-button px-4 py-2">
  Click Me
</button>
```

#### Method 2: Modify Global CSS
For base styles and CSS variables:

```css
/* packages/admin-spa/src/index.css */
@tailwind base;
@tailwind components;
@tailwind utilities;

:root {
  font-family: 'Your Custom Font', Inter, system-ui, sans-serif;
  --primary-color: #2563eb;
  --success-color: #16a34a;
  --danger-color: #dc2626;
}

/* Add custom component styles */
@layer components {
  .btn-primary {
    @apply px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors;
  }

  .card {
    @apply bg-white border border-gray-200 rounded-lg p-6 hover:shadow-lg transition-shadow;
  }
}
```

### Mobile App (React Native)

For the mobile app, create a centralized theme file:

#### Step 1: Create Theme File
```javascript
// packages/app/src/theme.ts
export const colors = {
  primary: '#2563eb',
  primaryDark: '#1d4ed8',
  success: '#16a34a',
  successDark: '#15803d',
  danger: '#dc2626',
  dangerDark: '#b91c1c',

  gray: {
    50: '#f9fafb',
    100: '#f3f4f6',
    200: '#e5e7eb',
    400: '#9ca3af',
    500: '#6b7280',
    700: '#374151',
    900: '#111827',
  },

  urgency: {
    urgent: '#fee2e2',
    medium: '#fef3c7',
    low: '#dbeafe',
  },

  white: '#ffffff',
  background: '#f3f4f6',
};

export const typography = {
  fontSizes: {
    xs: 12,
    sm: 14,
    base: 16,
    lg: 18,
    xl: 20,
    '2xl': 24,
    '3xl': 28,
  },
  fontWeights: {
    regular: '400',
    medium: '500',
    semibold: '600',
    bold: '700',
  },
  lineHeights: {
    tight: 1.25,
    normal: 1.5,
    relaxed: 1.75,
  },
};

export const spacing = {
  xs: 4,
  sm: 8,
  md: 12,
  lg: 16,
  xl: 20,
  '2xl': 24,
  '3xl': 32,
};

export const borderRadius = {
  sm: 8,
  md: 12,
  lg: 16,
};

export const shadows = {
  card: {
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: 0.1,
    shadowRadius: 4,
    elevation: 3,
  },
};
```

#### Step 2: Use Theme in Components
```javascript
// Example: packages/app/src/screens/NeedListScreen.tsx
import { colors, typography, spacing, borderRadius, shadows } from '../theme';

const styles = StyleSheet.create({
  card: {
    backgroundColor: colors.white,
    borderRadius: borderRadius.md,
    padding: spacing.lg,
    marginBottom: spacing.lg,
    ...shadows.card,
  },
  title: {
    fontSize: typography.fontSizes.xl,
    fontWeight: typography.fontWeights.semibold,
    color: colors.gray[900],
  },
  primaryButton: {
    backgroundColor: colors.primary,
    padding: spacing.lg,
    borderRadius: borderRadius.md,
  },
});
```

## File Reference

### Admin Web App Files to Modify

| File | Purpose |
|------|---------|
| `packages/admin-spa/tailwind.config.js` | Theme configuration (colors, fonts, spacing) |
| `packages/admin-spa/src/index.css` | Global styles, custom components |
| `packages/admin-spa/src/pages/NeedApprovalQueue.tsx` | Admin dashboard component |
| `packages/admin-spa/src/App.tsx` | Main app wrapper |

### Mobile App Files to Modify

| File | Purpose |
|------|---------|
| `packages/app/src/theme.ts` | **CREATE THIS** - Central theme configuration |
| `packages/app/src/screens/NeedListScreen.tsx` | Volunteer need list screen |
| `packages/app/src/screens/NeedDetailScreen.tsx` | Need detail screen |
| `packages/app/App.tsx` | Main app wrapper |

## Design Principles

The current design follows these principles:

1. **Clarity**: High contrast, readable typography, clear visual hierarchy
2. **Accessibility**: WCAG 2.1 AA compliant color contrasts
3. **Consistency**: Unified color palette and spacing system across platforms
4. **Mobile-First**: Touch-friendly hit areas (minimum 44px), responsive layouts
5. **Performance**: Lightweight styling, optimized for fast rendering

## Testing Your Changes

### Admin Web App
```bash
cd packages/admin-spa
npm run dev
# Open http://localhost:5173
```

### Mobile App
```bash
cd packages/app
npm start
# Press 'w' for web preview
# Press 'i' for iOS simulator
# Press 'a' for Android emulator
```

## Design Tokens Checklist

When creating a custom theme, consider updating:

- [ ] Primary brand colors
- [ ] Success/error/warning colors
- [ ] Typography scale
- [ ] Font families
- [ ] Spacing scale
- [ ] Border radius values
- [ ] Shadow/elevation styles
- [ ] Button styles
- [ ] Card styles
- [ ] Badge/chip styles
- [ ] Input field styles

## Resources

- **Tailwind CSS Documentation**: https://tailwindcss.com/docs
- **React Native StyleSheet**: https://reactnative.dev/docs/stylesheet
- **Expo Documentation**: https://docs.expo.dev/
- **Color Palette Generator**: https://coolors.co/
- **Contrast Checker**: https://webaim.org/resources/contrastchecker/

## Getting Help

If you need assistance with customization:

1. Check the component files to see how styles are currently applied
2. Test changes in both the web and mobile apps for consistency
3. Use browser DevTools or React Native Debugger to inspect styles
4. Refer to the Tailwind CSS or React Native documentation for specific syntax

## Next Steps

1. **Create a brand color palette** - Define your primary, secondary, and accent colors
2. **Update tailwind.config.js** - Extend the theme with your custom colors
3. **Create theme.ts** - Set up the mobile theme file with matching colors
4. **Test across platforms** - Ensure consistency between web and mobile
5. **Document your changes** - Keep a record of custom design tokens
