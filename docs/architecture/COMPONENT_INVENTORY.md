# Component Inventory

Visual guide to all UI components in Minnesota Digital Aid with their styling specifications.

## Admin Web App Components

### Buttons

#### Primary Button
**Location**: `packages/admin-spa/src/pages/NeedApprovalQueue.tsx:104`
```html
<button className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700">
  View Details
</button>
```
- **Background**: `#2563eb` (blue-600)
- **Hover**: `#1d4ed8` (blue-700)
- **Text**: White
- **Padding**: 16px horizontal, 8px vertical
- **Border Radius**: 4px

#### Success Button
**Location**: `packages/admin-spa/src/pages/NeedApprovalQueue.tsx:109`
```html
<button className="px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700">
  ✓ Approve
</button>
```
- **Background**: `#16a34a` (green-600)
- **Hover**: `#15803d` (green-700)
- **Text**: White
- **Icon**: ✓ checkmark

#### Danger Button
**Location**: `packages/admin-spa/src/pages/NeedApprovalQueue.tsx:115`
```html
<button className="px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700">
  ✗ Reject
</button>
```
- **Background**: `#dc2626` (red-600)
- **Hover**: `#b91c1c` (red-700)
- **Text**: White
- **Icon**: ✗ x-mark

#### Secondary Button
**Location**: `packages/admin-spa/src/pages/NeedApprovalQueue.tsx:194`
```html
<button className="px-4 py-2 bg-gray-300 text-gray-700 rounded hover:bg-gray-400">
  Cancel
</button>
```
- **Background**: `#d1d5db` (gray-300)
- **Hover**: `#9ca3af` (gray-400)
- **Text**: `#374151` (gray-700)

### Cards

#### Need Card
**Location**: `packages/admin-spa/src/pages/NeedApprovalQueue.tsx:72-74`
```html
<div className="bg-white border border-gray-200 rounded-lg p-6 hover:shadow-lg transition-shadow">
  <!-- Card content -->
</div>
```
- **Background**: White
- **Border**: 1px solid `#e5e7eb` (gray-200)
- **Border Radius**: 8px
- **Padding**: 24px
- **Hover Effect**: Shadow-lg
- **Transition**: Shadow with 150ms ease

### Badges

#### Badge Base
**Location**: `packages/admin-spa/src/pages/NeedApprovalQueue.tsx:79`
```html
<span className="text-xs font-medium px-2 py-1 bg-gray-100 rounded">
  👤 User
</span>
```
- **Background**: `#f3f4f6` (gray-100)
- **Text**: `12px`, weight 500
- **Padding**: 8px horizontal, 4px vertical
- **Border Radius**: 4px

#### Urgency Badge - Urgent
**Location**: `packages/admin-spa/src/pages/NeedApprovalQueue.tsx:84`
```html
<span className="text-xs font-medium px-2 py-1 rounded bg-red-100 text-red-700">
  urgent
</span>
```
- **Background**: `#fee2e2` (red-100)
- **Text**: `#b91c1c` (red-700)

#### Urgency Badge - Medium
```html
<span className="text-xs font-medium px-2 py-1 rounded bg-yellow-100 text-yellow-700">
  medium
</span>
```
- **Background**: `#fef3c7` (yellow-100)
- **Text**: `#a16207` (yellow-700)

#### Urgency Badge - Low
```html
<span className="text-xs font-medium px-2 py-1 rounded bg-blue-100 text-blue-700">
  low
</span>
```
- **Background**: `#dbeafe` (blue-100)
- **Text**: `#1d4ed8` (blue-700)

### Modal

#### Modal Overlay
**Location**: `packages/admin-spa/src/pages/NeedApprovalQueue.tsx:128`
```html
<div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4">
```
- **Background**: `rgba(0, 0, 0, 0.5)`
- **Position**: Fixed, full screen
- **Display**: Flex, centered

#### Modal Content
**Location**: `packages/admin-spa/src/pages/NeedApprovalQueue.tsx:129`
```html
<div className="bg-white rounded-lg max-w-2xl w-full max-h-[80vh] overflow-y-auto p-6">
```
- **Background**: White
- **Border Radius**: 8px
- **Max Width**: 672px (2xl)
- **Max Height**: 80% viewport height
- **Padding**: 24px
- **Overflow**: Scroll on Y-axis

### Typography

#### Page Heading (H1)
**Location**: `packages/admin-spa/src/pages/NeedApprovalQueue.tsx:63`
```html
<h1 className="text-3xl font-bold mb-8">Need Approval Queue</h1>
```
- **Size**: 30px (3xl)
- **Weight**: 700 (bold)
- **Margin Bottom**: 32px

#### Card Title (H3)
**Location**: `packages/admin-spa/src/pages/NeedApprovalQueue.tsx:92`
```html
<h3 className="text-xl font-semibold mb-1">{need.title}</h3>
```
- **Size**: 20px (xl)
- **Weight**: 600 (semibold)
- **Margin Bottom**: 4px

#### Organization Name
**Location**: `packages/admin-spa/src/pages/NeedApprovalQueue.tsx:93`
```html
<p className="text-sm text-gray-600 mb-2">{need.organizationName}</p>
```
- **Size**: 14px (sm)
- **Color**: `#4b5563` (gray-600)
- **Margin Bottom**: 8px

#### Body Text
**Location**: `packages/admin-spa/src/pages/NeedApprovalQueue.tsx:97`
```html
<p className="text-gray-700 mb-4">{need.tldr}</p>
```
- **Size**: 16px (base)
- **Color**: `#374151` (gray-700)
- **Margin Bottom**: 16px

#### Empty State Text
**Location**: `packages/admin-spa/src/pages/NeedApprovalQueue.tsx:66`
```html
<div className="text-gray-500 text-center py-12">
  No pending needs to review
</div>
```
- **Color**: `#6b7280` (gray-500)
- **Alignment**: Center
- **Padding**: 48px vertical

### Links
**Location**: `packages/admin-spa/src/pages/NeedApprovalQueue.tsx:170`
```html
<a href={url} className="text-blue-600 hover:underline">
  {selectedNeed.contactInfo.website}
</a>
```
- **Color**: `#2563eb` (blue-600)
- **Hover**: Underline
- **Text Decoration**: None (default)

---

## Mobile App Components

### Buttons

#### Primary Button
**Location**: `packages/app/src/screens/NeedDetailScreen.tsx:224-232`
```javascript
interestedButton: {
  backgroundColor: '#2563eb',
  padding: 16,
  borderRadius: 12,
  alignItems: 'center',
  marginTop: 12,
  marginBottom: 32,
}
interestedButtonText: {
  color: 'white',
  fontSize: 18,
  fontWeight: '600',
}
```
- **Background**: `#2563eb` (primary blue)
- **Padding**: 16px all sides
- **Border Radius**: 12px
- **Text**: White, 18px, weight 600

#### Retry Button
**Location**: `packages/app/src/screens/NeedListScreen.tsx:162-167`
```javascript
retryButton: {
  paddingHorizontal: 20,
  paddingVertical: 10,
  backgroundColor: '#2563eb',
  borderRadius: 8,
}
retryText: {
  color: 'white',
  fontSize: 16,
  fontWeight: '600',
}
```
- **Background**: `#2563eb`
- **Padding**: 20px horizontal, 10px vertical
- **Border Radius**: 8px

#### Contact Button
**Location**: `packages/app/src/screens/NeedDetailScreen.tsx:211-218`
```javascript
contactButton: {
  backgroundColor: 'white',
  padding: 16,
  borderRadius: 8,
  marginBottom: 8,
  borderWidth: 1,
  borderColor: '#e5e7eb',
}
contactButtonText: {
  fontSize: 16,
  color: '#2563eb',
  fontWeight: '500',
}
```
- **Background**: White
- **Border**: 1px solid `#e5e7eb`
- **Border Radius**: 8px
- **Text**: `#2563eb`, 16px, weight 500

#### Back Button
**Location**: `packages/app/src/screens/NeedDetailScreen.tsx:242-247`
```javascript
backButton: {
  paddingHorizontal: 20,
  paddingVertical: 10,
  backgroundColor: '#6b7280',
  borderRadius: 8,
}
backButtonText: {
  color: 'white',
  fontSize: 16,
  fontWeight: '600',
}
```
- **Background**: `#6b7280` (gray-500)
- **Text**: White, 16px, weight 600

### Cards

#### Need Card
**Location**: `packages/app/src/screens/NeedListScreen.tsx:105-115`
```javascript
card: {
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
- **Background**: White
- **Border Radius**: 12px
- **Padding**: 16px
- **Shadow**: iOS + Android elevation
- **Margin Bottom**: 16px

### Badges

#### Urgency Badge - Base
**Location**: `packages/app/src/screens/NeedListScreen.tsx:127-132`
```javascript
urgencyBadge: {
  paddingHorizontal: 8,
  paddingVertical: 4,
  borderRadius: 12,
  backgroundColor: '#fef3c7',
}
```
- **Background**: `#fef3c7` (yellow-100)
- **Padding**: 8px horizontal, 4px vertical
- **Border Radius**: 12px

#### Urgency Badge - Urgent
**Location**: `packages/app/src/screens/NeedListScreen.tsx:133-135`
```javascript
urgencyUrgent: {
  backgroundColor: '#fee2e2',
}
```
- **Background**: `#fee2e2` (red-100)

#### Urgency Text
**Location**: `packages/app/src/screens/NeedListScreen.tsx:136-140`
```javascript
urgencyText: {
  fontSize: 12,
  fontWeight: '600',
  color: '#92400e',
}
```
- **Size**: 12px
- **Weight**: 600
- **Color**: `#92400e` (amber-700)

### Typography

#### Large Title
**Location**: `packages/app/src/screens/NeedDetailScreen.tsx:178-183`
```javascript
title: {
  fontSize: 28,
  fontWeight: '700',
  color: '#111827',
  marginBottom: 12,
}
```
- **Size**: 28px
- **Weight**: 700
- **Color**: `#111827` (gray-900)

#### Card Title
**Location**: `packages/app/src/screens/NeedListScreen.tsx:141-146`
```javascript
title: {
  fontSize: 18,
  fontWeight: '600',
  color: '#111827',
  marginBottom: 8,
}
```
- **Size**: 18px
- **Weight**: 600
- **Color**: `#111827`

#### Section Title
**Location**: `packages/app/src/screens/NeedDetailScreen.tsx:192-197`
```javascript
sectionTitle: {
  fontSize: 18,
  fontWeight: '600',
  color: '#111827',
  marginBottom: 12,
}
```
- **Size**: 18px
- **Weight**: 600

#### Organization Name
**Location**: `packages/app/src/screens/NeedListScreen.tsx:122-126`
```javascript
organizationName: {
  fontSize: 14,
  color: '#6b7280',
  fontWeight: '500',
}
```
- **Size**: 14px
- **Color**: `#6b7280` (gray-500)
- **Weight**: 500

#### Body Text (TLDR)
**Location**: `packages/app/src/screens/NeedListScreen.tsx:152-156`
```javascript
tldr: {
  fontSize: 14,
  color: '#4b5563',
  lineHeight: 20,
}
```
- **Size**: 14px
- **Color**: `#4b5563` (gray-600)
- **Line Height**: 20px

#### Description Text
**Location**: `packages/app/src/screens/NeedDetailScreen.tsx:206-210`
```javascript
description: {
  fontSize: 16,
  color: '#374151',
  lineHeight: 24,
}
```
- **Size**: 16px
- **Color**: `#374151` (gray-700)
- **Line Height**: 24px

#### Location Text
**Location**: `packages/app/src/screens/NeedListScreen.tsx:147-151`
```javascript
location: {
  fontSize: 14,
  color: '#6b7280',
  marginBottom: 8,
}
```
- **Size**: 14px
- **Color**: `#6b7280`
- **Emoji**: 📍

#### Error Text
**Location**: `packages/app/src/screens/NeedListScreen.tsx:157-161`
```javascript
errorText: {
  fontSize: 16,
  color: '#ef4444',
  marginBottom: 16,
}
```
- **Size**: 16px
- **Color**: `#ef4444` (red-500)

#### Empty State Text
**Location**: `packages/app/src/screens/NeedListScreen.tsx:173-176`
```javascript
emptyText: {
  fontSize: 16,
  color: '#9ca3af',
}
```
- **Size**: 16px
- **Color**: `#9ca3af` (gray-400)

### Layout Components

#### Container
**Location**: `packages/app/src/screens/NeedListScreen.tsx:92-95`
```javascript
container: {
  flex: 1,
  backgroundColor: '#f3f4f6',
}
```
- **Background**: `#f3f4f6` (gray-100)
- **Flex**: 1 (full height)

#### Content Container
**Location**: `packages/app/src/screens/NeedDetailScreen.tsx:150-152`
```javascript
content: {
  padding: 16,
}
```
- **Padding**: 16px all sides

#### Center Container
**Location**: `packages/app/src/screens/NeedListScreen.tsx:96-101`
```javascript
center: {
  flex: 1,
  justifyContent: 'center',
  alignItems: 'center',
  padding: 20,
}
```
- **Alignment**: Centered vertically and horizontally
- **Padding**: 20px

#### List Container
**Location**: `packages/app/src/screens/NeedListScreen.tsx:102-104`
```javascript
list: {
  padding: 16,
}
```
- **Padding**: 16px

#### Section
**Location**: `packages/app/src/screens/NeedDetailScreen.tsx:189-191`
```javascript
section: {
  marginBottom: 24,
}
```
- **Margin Bottom**: 24px

### Loading Indicators
**Location**: `packages/app/src/screens/NeedDetailScreen.tsx:20`
```javascript
<ActivityIndicator size="large" color="#2563eb" />
```
- **Color**: `#2563eb` (primary blue)
- **Size**: Large

---

## Component Usage Matrix

| Component | Admin Web | Mobile App | Notes |
|-----------|-----------|------------|-------|
| Primary Button | ✅ | ✅ | Same color, different syntax |
| Success Button | ✅ | ❌ | Web only |
| Danger Button | ✅ | ❌ | Web only |
| Secondary Button | ✅ | ✅ | Different implementation |
| Need Card | ✅ | ✅ | Same design language |
| Urgency Badge | ✅ | ✅ | Same colors |
| Submission Type Badge | ✅ | ❌ | Web only |
| Modal | ✅ | ❌ | Web only |
| Activity Indicator | ❌ | ✅ | Mobile only |
| Contact Buttons | ✅ (links) | ✅ (buttons) | Different interaction |

## Interactive States

### Hover States (Web Only)
- **Buttons**: Darker shade of background color
- **Cards**: Shadow-lg elevation
- **Links**: Underline decoration

### Touch States (Mobile)
- **TouchableOpacity**: Default opacity reduction (0.2)
- **Active Press**: Visual feedback from native platform

### Focus States
- **Web**: Browser default outline
- **Mobile**: Native platform highlighting

## Accessibility Considerations

### Touch Targets (Mobile)
- **Minimum Size**: 44px × 44px
- **Padding**: Adequate spacing for fat fingers
- **Icon Buttons**: Larger padding area than visible icon

### Color Contrast
- All text meets WCAG 2.1 AA standards
- Primary blue on white: 8.59:1 (AAA)
- Gray text on white: Minimum 4.5:1

### Screen Reader Support
- Semantic HTML elements (web)
- AccessibilityLabel props (mobile - not yet implemented)

## Animation & Transitions

### Web Transitions
```css
transition-shadow    /* 150ms ease-in-out */
hover:shadow-lg     /* Shadow elevation on hover */
```

### Mobile Animations
- Native TouchableOpacity feedback
- ScrollView momentum scrolling
- ActivityIndicator spinning

## Design Pattern Notes

1. **Consistent Spacing**: Both platforms use multiples of 4px/8px
2. **Color Harmony**: Same color values across web and mobile
3. **Border Radius**: Slightly more rounded on mobile (12px vs 8px)
4. **Typography Scale**: Mobile uses slightly smaller sizes for cards
5. **Touch Targets**: Mobile has larger padding for better UX
6. **Shadows**: More pronounced on mobile (elevation 3) vs web (hover only)
