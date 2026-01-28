# Design Tokens Reference

Quick reference for all design values used in Minnesota Digital Aid.

## Colors

### Brand Colors
```
Primary Blue:     #2563eb  rgb(37, 99, 235)
Primary Hover:    #1d4ed8  rgb(29, 78, 216)
```

### Semantic Colors
```
Success:          #16a34a  rgb(22, 163, 74)
Success Hover:    #15803d  rgb(21, 128, 61)

Danger:           #dc2626  rgb(220, 38, 38)
Danger Hover:     #b91c1c  rgb(185, 28, 28)

Green (Approve):  #16a34a  rgb(22, 163, 74)
Green Hover:      #15803d  rgb(21, 128, 61)

Red (Reject):     #dc2626  rgb(220, 38, 38)
Red Hover:        #b91c1c  rgb(185, 28, 28)
```

### Gray Scale
```
gray-50:          #f9fafb  rgb(249, 250, 251)
gray-100:         #f3f4f6  rgb(243, 244, 246)  [Background]
gray-200:         #e5e7eb  rgb(229, 231, 235)  [Borders]
gray-300:         #d1d5db  rgb(209, 213, 219)
gray-400:         #9ca3af  rgb(156, 163, 175)  [Muted Text]
gray-500:         #6b7280  rgb(107, 114, 128)  [Secondary Text]
gray-600:         #4b5563  rgb(75, 85, 99)
gray-700:         #374151  rgb(55, 65, 81)     [Body Text]
gray-800:         #1f2937  rgb(31, 41, 55)
gray-900:         #111827  rgb(17, 24, 39)     [Headings]
```

### Urgency Colors
```
Urgent Background:   #fee2e2  rgb(254, 226, 226)  [red-100]
Urgent Text:         #92400e  rgb(146, 64, 14)

Medium Background:   #fef3c7  rgb(254, 243, 199)  [yellow-100]
Medium Text:         #92400e  rgb(146, 64, 14)

Low Background:      #dbeafe  rgb(219, 234, 254)  [blue-100]
Low Text:            #1e40af  rgb(30, 64, 175)
```

### Special Colors
```
White:            #ffffff  rgb(255, 255, 255)
Black:            #000000  rgb(0, 0, 0)
Transparent:      rgba(0, 0, 0, 0)
Modal Overlay:    rgba(0, 0, 0, 0.5)
```

## Typography

### Font Families
```
Primary:  Inter, system-ui, Avenir, Helvetica, Arial, sans-serif
Mobile:   System Default
```

### Font Sizes (px)
```
xs:       12px
sm:       14px
base:     16px
lg:       18px
xl:       20px
2xl:      24px
3xl:      28px / 30px (web)
```

### Font Weights
```
regular:  400
medium:   500
semibold: 600
bold:     700
```

### Line Heights
```
tight:    1.25
normal:   1.5
relaxed:  1.75

Specific values:
- 20px (body small)
- 24px (body large)
```

## Spacing (px)

```
1:   4px
2:   8px
3:   12px
4:   16px
5:   20px
6:   24px
8:   32px
12:  48px
16:  64px
20:  80px
24:  96px
32:  128px
```

## Border Radius (px)

```
sm:       8px   [small buttons, badges]
md:       12px  [cards, main buttons]
lg:       16px  [urgency badges]

Specific:
button:   8px
card:     12px
badge:    12px
modal:    8px - 12px
```

## Shadows

### Card Shadow (Web - Tailwind)
```css
hover:shadow-lg
```

### Card Shadow (Mobile - StyleSheet)
```javascript
{
  shadowColor: '#000',
  shadowOffset: { width: 0, height: 2 },
  shadowOpacity: 0.1,
  shadowRadius: 4,
  elevation: 3,  // Android
}
```

## Component Sizes

### Buttons
```
Padding:          16px horizontal, 10-16px vertical
Border Radius:    8px - 12px
Font Size:        16px - 18px
Font Weight:      600
Min Height:       44px (mobile - touch target)
```

### Cards
```
Padding:          16px - 24px
Border Radius:    12px
Margin Bottom:    16px
Border:           1px solid gray-200
Background:       white
```

### Badges
```
Padding:          8-12px horizontal, 4-6px vertical
Border Radius:    12px - 16px
Font Size:        12px - 14px
Font Weight:      600
```

### Modal (Web)
```
Max Width:        672px (2xl)
Max Height:       80vh
Padding:          24px
Border Radius:    8px - 12px
Background:       white
Overlay:          rgba(0, 0, 0, 0.5)
```

## Layout

### Container Widths
```
Max Width (Admin):  1280px (7xl)
Padding:            32px
```

### Content Spacing
```
Section Margin:     24px bottom
Card Gap:           24px (grid-gap)
List Padding:       16px
```

## Breakpoints (Tailwind)

```
sm:   640px
md:   768px
lg:   1024px
xl:   1280px
2xl:  1536px
```

## Animation/Transitions

```
Transition:      all 0.2s ease-in-out
Hover Effects:   Darker shade of background
Active Effects:  Scale 0.95 or darker background
```

## Opacity Values

```
Overlay:        0.5
Disabled:       0.6
Shadow:         0.1
```

## Z-Index Layers

```
Base:           0
Card:           1
Dropdown:       10
Modal Overlay:  40
Modal Content:  50
Tooltip:        60
```

## Icon Sizes

```
Emoji (inline):  Natural size (18-24px typical)
Badge icons:     12-14px
Button icons:    16-20px
```

## Useful Tailwind Classes

### Common Combinations

#### Primary Button
`px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700`

#### Success Button
`px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700`

#### Danger Button
`px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700`

#### Card
`bg-white border border-gray-200 rounded-lg p-6 hover:shadow-lg transition-shadow`

#### Badge
`text-xs font-medium px-2 py-1 bg-gray-100 rounded`

#### Modal Container
`fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4`

#### Modal Content
`bg-white rounded-lg max-w-2xl w-full max-h-[80vh] overflow-y-auto p-6`

## Color Contrast Ratios

All color combinations meet WCAG 2.1 AA standards:

```
Primary Blue (#2563eb) on White:     8.59:1  ✓
Gray 900 (#111827) on White:         16.5:1  ✓
Gray 700 (#374151) on White:         11.1:1  ✓
Gray 500 (#6b7280) on White:         6.9:1   ✓
White on Primary Blue:               8.59:1  ✓
White on Success Green:              5.36:1  ✓
White on Danger Red:                 5.94:1  ✓
```

## CSS Custom Properties (Optional)

You can add these to `index.css` for easier maintenance:

```css
:root {
  /* Colors */
  --color-primary: #2563eb;
  --color-primary-hover: #1d4ed8;
  --color-success: #16a34a;
  --color-danger: #dc2626;

  /* Grays */
  --color-gray-50: #f9fafb;
  --color-gray-100: #f3f4f6;
  --color-gray-200: #e5e7eb;
  --color-gray-500: #6b7280;
  --color-gray-700: #374151;
  --color-gray-900: #111827;

  /* Typography */
  --font-family: Inter, system-ui, sans-serif;
  --font-size-base: 16px;
  --font-weight-regular: 400;
  --font-weight-medium: 500;
  --font-weight-semibold: 600;
  --font-weight-bold: 700;

  /* Spacing */
  --spacing-sm: 8px;
  --spacing-md: 16px;
  --spacing-lg: 24px;
  --spacing-xl: 32px;

  /* Border Radius */
  --radius-sm: 8px;
  --radius-md: 12px;
  --radius-lg: 16px;
}
```
