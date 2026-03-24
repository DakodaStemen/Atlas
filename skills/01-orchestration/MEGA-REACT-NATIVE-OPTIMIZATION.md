---
name: Use expo-image for Optimized Images
description: ## Use expo-image for Optimized Images
 
 Use `expo-image` instead of React Native's `Image`. It provides memory-efficient caching, blurhash placeholders, progressive loading, and better performance for lists.
tags: images, performance, expo-image, ui
---

## Use expo-image for Optimized Images

Use `expo-image` instead of React Native's `Image`. It provides memory-efficient caching, blurhash placeholders, progressive loading, and better performance for lists.

### Incorrect (React Native Image)

```tsx
import { Image } from 'react-native'

function Avatar({ url }: { url: string }) {
  return <Image source={{ uri: url }} style={styles.avatar} />
}
```

#### Correct (expo-image)

```tsx
import { Image } from 'expo-image'

function Avatar({ url }: { url: string }) {
  return <Image source={{ uri: url }} style={styles.avatar} />
}
```

#### With blurhash placeholder

```tsx
<Image
  source={{ uri: url }}
  placeholder={{ blurhash: 'LGF5]+Yk^6#M@-5c,1J5@[or[Q6.' }}
  contentFit="cover"
  transition={200}
  style={styles.image}
/>
```

#### With priority and caching

```tsx
<Image
  source={{ uri: url }}
  priority="high"
  cachePolicy="memory-disk"
  style={styles.hero}
/>
```

#### Key props

- `placeholder` — Blurhash or thumbnail while loading
- `contentFit` — `cover`, `contain`, `fill`, `scale-down`
- `transition` — Fade-in duration (ms)
- `priority` — `low`, `normal`, `high`
- `cachePolicy` — `memory`, `disk`, `memory-disk`, `none`
- `recyclingKey` — Unique key for list recycling

For cross-platform (web + native), use `SolitoImage` from `solito/image` which uses `expo-image` under the hood.

Reference: [expo-image](https://docs.expo.dev/versions/latest/sdk/image/)

## When to use

Use when the user asks about or needs: Use expo-image for Optimized Images.
﻿---
name: Use Galeria for Image Galleries and Lightbox
description: ## Use Galeria for Image Galleries and Lightbox
 
 For image galleries with lightbox (tap to fullscreen), use `@nandorojo/galeria`.
tags: images, gallery, lightbox, expo-image, ui
---

## Use Galeria for Image Galleries and Lightbox

For image galleries with lightbox (tap to fullscreen), use `@nandorojo/galeria`.
It provides native shared element transitions with pinch-to-zoom, double-tap
zoom, and pan-to-close. Works with any image component including `expo-image`.

### Incorrect (custom modal implementation)

```tsx
function ImageGallery({ urls }: { urls: string[] }) {
  const [selected, setSelected] = useState<string | null>(null)

  return (
    <>
      {urls.map((url) => (
        <Pressable key={url} onPress={() => setSelected(url)}>
          <Image source={{ uri: url }} style={styles.thumbnail} />
        </Pressable>
      ))}
      <Modal visible={!!selected} onRequestClose={() => setSelected(null)}>
        <Image source={{ uri: selected! }} style={styles.fullscreen} />
      </Modal>
    </>
  )
}
```

#### Correct (Galeria with expo-image)

```tsx
import { Galeria } from '@nandorojo/galeria'
import { Image } from 'expo-image'

function ImageGallery({ urls }: { urls: string[] }) {
  return (
    <Galeria urls={urls}>
      {urls.map((url, index) => (
        <Galeria.Image index={index} key={url}>
          <Image source={{ uri: url }} style={styles.thumbnail} />
        </Galeria.Image>
      ))}
    </Galeria>
  )
}
```

#### Single image

```tsx
import { Galeria } from '@nandorojo/galeria'
import { Image } from 'expo-image'

function Avatar({ url }: { url: string }) {
  return (
    <Galeria urls={[url]}>
      <Galeria.Image>
        <Image source={{ uri: url }} style={styles.avatar} />
      </Galeria.Image>
    </Galeria>
  )
}
```

#### With low-res thumbnails and high-res fullscreen

```tsx
<Galeria urls={highResUrls}>
  {lowResUrls.map((url, index) => (
    <Galeria.Image index={index} key={url}>
      <Image source={{ uri: url }} style={styles.thumbnail} />
    </Galeria.Image>
  ))}
</Galeria>
```

#### With FlashList

```tsx
<Galeria urls={urls}>
  <FlashList
    data={urls}
    renderItem={({ item, index }) => (
      <Galeria.Image index={index}>
        <Image source={{ uri: item }} style={styles.thumbnail} />
      </Galeria.Image>
    )}
    numColumns={3}
    estimatedItemSize={100}
  />
</Galeria>
```

Works with `expo-image`, `SolitoImage`, `react-native` Image, or any image
component.

Reference: [Galeria](https://github.com/nandorojo/galeria)

## When to use

Use when the user asks about or needs: Use Galeria for Image Galleries and Lightbox.
﻿---
name: Use Native Menus for Dropdowns and Context Menus
description: ## Use Native Menus for Dropdowns and Context Menus
 
 Use native platform menus instead of custom JS implementations. Native menus
tags: user-interface, menus, context-menus, zeego, accessibility
---

## Use Native Menus for Dropdowns and Context Menus

Use native platform menus instead of custom JS implementations. Native menus
provide built-in accessibility, consistent platform UX, and better performance.
Use [zeego](https://zeego.dev) for cross-platform native menus.

### Incorrect (custom JS menu)

```tsx
import { useState } from 'react'
import { View, Pressable, Text } from 'react-native'

function MyMenu() {
  const [open, setOpen] = useState(false)

  return (
    <View>
      <Pressable onPress={() => setOpen(!open)}>
        <Text>Open Menu</Text>
      </Pressable>
      {open && (
        <View style={{ position: 'absolute', top: 40 }}>
          <Pressable onPress={() => console.log('edit')}>
            <Text>Edit</Text>
          </Pressable>
          <Pressable onPress={() => console.log('delete')}>
            <Text>Delete</Text>
          </Pressable>
        </View>
      )}
    </View>
  )
}
```

#### Correct (native menu with zeego)

```tsx
import * as DropdownMenu from 'zeego/dropdown-menu'

function MyMenu() {
  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger>
        <Pressable>
          <Text>Open Menu</Text>
        </Pressable>
      </DropdownMenu.Trigger>

      <DropdownMenu.Content>
        <DropdownMenu.Item key='edit' onSelect={() => console.log('edit')}>
          <DropdownMenu.ItemTitle>Edit</DropdownMenu.ItemTitle>
        </DropdownMenu.Item>

        <DropdownMenu.Item
          key='delete'
          destructive
          onSelect={() => console.log('delete')}
        >
          <DropdownMenu.ItemTitle>Delete</DropdownMenu.ItemTitle>
        </DropdownMenu.Item>
      </DropdownMenu.Content>
    </DropdownMenu.Root>
  )
}
```

#### Context menu (long-press)

```tsx
import * as ContextMenu from 'zeego/context-menu'

function MyContextMenu() {
  return (
    <ContextMenu.Root>
      <ContextMenu.Trigger>
        <View style={{ padding: 20 }}>
          <Text>Long press me</Text>
        </View>
      </ContextMenu.Trigger>

      <ContextMenu.Content>
        <ContextMenu.Item key='copy' onSelect={() => console.log('copy')}>
          <ContextMenu.ItemTitle>Copy</ContextMenu.ItemTitle>
        </ContextMenu.Item>

        <ContextMenu.Item key='paste' onSelect={() => console.log('paste')}>
          <ContextMenu.ItemTitle>Paste</ContextMenu.ItemTitle>
        </ContextMenu.Item>
      </ContextMenu.Content>
    </ContextMenu.Root>
  )
}
```

#### Checkbox items

```tsx
import * as DropdownMenu from 'zeego/dropdown-menu'

function SettingsMenu() {
  const [notifications, setNotifications] = useState(true)

  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger>
        <Pressable>
          <Text>Settings</Text>
        </Pressable>
      </DropdownMenu.Trigger>

      <DropdownMenu.Content>
        <DropdownMenu.CheckboxItem
          key='notifications'
          value={notifications}
          onValueChange={() => setNotifications((prev) => !prev)}
        >
          <DropdownMenu.ItemIndicator />
          <DropdownMenu.ItemTitle>Notifications</DropdownMenu.ItemTitle>
        </DropdownMenu.CheckboxItem>
      </DropdownMenu.Content>
    </DropdownMenu.Root>
  )
}
```

#### Submenus

```tsx
import * as DropdownMenu from 'zeego/dropdown-menu'

function MenuWithSubmenu() {
  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger>
        <Pressable>
          <Text>Options</Text>
        </Pressable>
      </DropdownMenu.Trigger>

      <DropdownMenu.Content>
        <DropdownMenu.Item key='home' onSelect={() => console.log('home')}>
          <DropdownMenu.ItemTitle>Home</DropdownMenu.ItemTitle>
        </DropdownMenu.Item>

        <DropdownMenu.Sub>
          <DropdownMenu.SubTrigger key='more'>
            <DropdownMenu.ItemTitle>More Options</DropdownMenu.ItemTitle>
          </DropdownMenu.SubTrigger>

          <DropdownMenu.SubContent>
            <DropdownMenu.Item key='settings'>
              <DropdownMenu.ItemTitle>Settings</DropdownMenu.ItemTitle>
            </DropdownMenu.Item>

            <DropdownMenu.Item key='help'>
              <DropdownMenu.ItemTitle>Help</DropdownMenu.ItemTitle>
            </DropdownMenu.Item>
          </DropdownMenu.SubContent>
        </DropdownMenu.Sub>
      </DropdownMenu.Content>
    </DropdownMenu.Root>
  )
}
```

Reference: [Zeego Documentation](https://zeego.dev/components/dropdown-menu)

## When to use

Use when the user asks about or needs: Use Native Menus for Dropdowns and Context Menus.
﻿---
name: Use Native Modals Over JS-Based Bottom Sheets
description: ## Use Native Modals Over JS-Based Bottom Sheets
 
 Use native `<Modal>` with `presentationStyle="formSheet"` or React Navigation
tags: modals, bottom-sheet, native, react-navigation
---

## Use Native Modals Over JS-Based Bottom Sheets

Use native `<Modal>` with `presentationStyle="formSheet"` or React Navigation
v7's native form sheet instead of JS-based bottom sheet libraries. Native modals
have built-in gestures, accessibility, and better performance. Rely on native UI
for low-level primitives.

### Incorrect (JS-based bottom sheet)

```tsx
import BottomSheet from 'custom-js-bottom-sheet'

function MyScreen() {
  const sheetRef = useRef<BottomSheet>(null)

  return (
    <View style={{ flex: 1 }}>
      <Button onPress={() => sheetRef.current?.expand()} title='Open' />
      <BottomSheet ref={sheetRef} snapPoints={['50%', '90%']}>
        <View>
          <Text>Sheet content</Text>
        </View>
      </BottomSheet>
    </View>
  )
}
```

#### Correct (native Modal with formSheet)

```tsx
import { Modal, View, Text, Button } from 'react-native'

function MyScreen() {
  const [visible, setVisible] = useState(false)

  return (
    <View style={{ flex: 1 }}>
      <Button onPress={() => setVisible(true)} title='Open' />
      <Modal
        visible={visible}
        presentationStyle='formSheet'
        animationType='slide'
        onRequestClose={() => setVisible(false)}
      >
        <View>
          <Text>Sheet content</Text>
        </View>
      </Modal>
    </View>
  )
}
```

#### Correct (React Navigation v7 native form sheet)

```tsx
// In your navigator
<Stack.Screen
  name='Details'
  component={DetailsScreen}
  options={{
    presentation: 'formSheet',
    sheetAllowedDetents: 'fitToContents',
  }}
/>
```

Native modals provide swipe-to-dismiss, proper keyboard avoidance, and
accessibility out of the box.

## When to use

Use when the user asks about or needs: Use Native Modals Over JS-Based Bottom Sheets.
﻿---
name: Use Native Navigators for Navigation
description: ## Use Native Navigators for Navigation
 
 Always use native navigators instead of JS-based ones. Native navigators use
tags: navigation, react-navigation, expo-router, native-stack, tabs
---

## Use Native Navigators for Navigation

Always use native navigators instead of JS-based ones. Native navigators use
platform APIs (UINavigationController on iOS, Fragment on Android) for better
performance and native behavior.

**For stacks:** Use `@react-navigation/native-stack` or expo-router's default
stack (which uses native-stack). Avoid `@react-navigation/stack`.

**For tabs:** Use `react-native-bottom-tabs` (native) or expo-router's native
tabs. Avoid `@react-navigation/bottom-tabs` when native feel matters.

### Stack Navigation

#### Incorrect (JS stack navigator)

```tsx
import { createStackNavigator } from '@react-navigation/stack'

const Stack = createStackNavigator()

function App() {
  return (
    <Stack.Navigator>
      <Stack.Screen name='Home' component={HomeScreen} />
      <Stack.Screen name='Details' component={DetailsScreen} />
    </Stack.Navigator>
  )
}
```

#### Correct (native stack with react-navigation)

```tsx
import { createNativeStackNavigator } from '@react-navigation/native-stack'

const Stack = createNativeStackNavigator()

function App() {
  return (
    <Stack.Navigator>
      <Stack.Screen name='Home' component={HomeScreen} />
      <Stack.Screen name='Details' component={DetailsScreen} />
    </Stack.Navigator>
  )
}
```

#### Correct (expo-router uses native stack by default)

```tsx
// app/_layout.tsx
import { Stack } from 'expo-router'

export default function Layout() {
  return <Stack />
}
```

### Tab Navigation

#### Incorrect (JS bottom tabs)

```tsx
import { createBottomTabNavigator } from '@react-navigation/bottom-tabs'

const Tab = createBottomTabNavigator()

function App() {
  return (
    <Tab.Navigator>
      <Tab.Screen name='Home' component={HomeScreen} />
      <Tab.Screen name='Settings' component={SettingsScreen} />
    </Tab.Navigator>
  )
}
```

#### Correct (native bottom tabs with react-navigation)

```tsx
import { createNativeBottomTabNavigator } from '@bottom-tabs/react-navigation'

const Tab = createNativeBottomTabNavigator()

function App() {
  return (
    <Tab.Navigator>
      <Tab.Screen
        name='Home'
        component={HomeScreen}
        options={{
          tabBarIcon: () => ({ sfSymbol: 'house' }),
        }}
      />
      <Tab.Screen
        name='Settings'
        component={SettingsScreen}
        options={{
          tabBarIcon: () => ({ sfSymbol: 'gear' }),
        }}
      />
    </Tab.Navigator>
  )
}
```

#### Correct (expo-router native tabs)

```tsx
// app/(tabs)/_layout.tsx
import { NativeTabs } from 'expo-router/unstable-native-tabs'

export default function TabLayout() {
  return (
    <NativeTabs>
      <NativeTabs.Trigger name='index'>
        <NativeTabs.Trigger.Label>Home</NativeTabs.Trigger.Label>
        <NativeTabs.Trigger.Icon sf='house.fill' md='home' />
      </NativeTabs.Trigger>
      <NativeTabs.Trigger name='settings'>
        <NativeTabs.Trigger.Label>Settings</NativeTabs.Trigger.Label>
        <NativeTabs.Trigger.Icon sf='gear' md='settings' />
      </NativeTabs.Trigger>
    </NativeTabs>
  )
}
```

On iOS, native tabs automatically enable `contentInsetAdjustmentBehavior` on the
first `ScrollView` at the root of each tab screen, so content scrolls correctly
behind the translucent tab bar. If you need to disable this, use
`disableAutomaticContentInsets` on the trigger.

### Prefer Native Header Options Over Custom Components

#### Incorrect (custom header component)

```tsx
<Stack.Screen
  name='Profile'
  component={ProfileScreen}
  options={{
    header: () => <CustomHeader title='Profile' />,
  }}
/>
```

#### Correct (native header options)

```tsx
<Stack.Screen
  name='Profile'
  component={ProfileScreen}
  options={{
    title: 'Profile',
    headerLargeTitleEnabled: true,
    headerSearchBarOptions: {
      placeholder: 'Search',
    },
  }}
/>
```

Native headers support iOS large titles, search bars, blur effects, and proper
safe area handling automatically.

### Why Native Navigators

- **Performance**: Native transitions and gestures run on the UI thread
- **Platform behavior**: Automatic iOS large titles, Android material design
- **System integration**: Scroll-to-top on tab tap, PiP avoidance, proper safe
  areas
- **Accessibility**: Platform accessibility features work automatically

Reference:

- [React Navigation Native Stack](https://reactnavigation.org/docs/native-stack-navigator)
- [React Native Bottom Tabs with React Navigation](https://oss.callstack.com/react-native-bottom-tabs/docs/guides/usage-with-react-navigation)
- [React Native Bottom Tabs with Expo Router](https://oss.callstack.com/react-native-bottom-tabs/docs/guides/usage-with-expo-router)
- [Expo Router Native Tabs](https://docs.expo.dev/router/advanced/native-tabs)

## When to use

Use when the user asks about or needs: Use Native Navigators for Navigation.
﻿---
name: Use Pressable Instead of Touchable Components
description: ## Use Pressable Instead of Touchable Components
 
 Never use `TouchableOpacity` or `TouchableHighlight`. Use `Pressable` from
tags: ui, pressable, touchable, gestures
---

## Use Pressable Instead of Touchable Components

Never use `TouchableOpacity` or `TouchableHighlight`. Use `Pressable` from
`react-native` or `react-native-gesture-handler` instead.

### Incorrect (legacy Touchable components)

```tsx
import { TouchableOpacity } from 'react-native'

function MyButton({ onPress }: { onPress: () => void }) {
  return (
    <TouchableOpacity onPress={onPress} activeOpacity={0.7}>
      <Text>Press me</Text>
    </TouchableOpacity>
  )
}
```

#### Correct (Pressable)

```tsx
import { Pressable } from 'react-native'

function MyButton({ onPress }: { onPress: () => void }) {
  return (
    <Pressable onPress={onPress}>
      <Text>Press me</Text>
    </Pressable>
  )
}
```

#### Correct (Pressable from gesture handler for lists)

```tsx
import { Pressable } from 'react-native-gesture-handler'

function ListItem({ onPress }: { onPress: () => void }) {
  return (
    <Pressable onPress={onPress}>
      <Text>Item</Text>
    </Pressable>
  )
}
```

Use `react-native-gesture-handler` Pressable inside scrollable lists for better
gesture coordination, as long as you are using the ScrollView from
`react-native-gesture-handler` as well.

**For animated press states (scale, opacity changes):** Use `GestureDetector`
with Reanimated shared values instead of Pressable's style callback. See the
`animation-gesture-detector-press` rule.

## When to use

Use when the user asks about or needs: Use Pressable Instead of Touchable Components.
﻿---
name: Use GestureDetector for Animated Press States
description: ## Use GestureDetector for Animated Press States
 
 For animated press states (scale, opacity on press), use `GestureDetector` with
tags: animation, gestures, press, reanimated
---

## Use GestureDetector for Animated Press States

For animated press states (scale, opacity on press), use `GestureDetector` with
`Gesture.Tap()` and shared values instead of Pressable's
`onPressIn`/`onPressOut`. Gesture callbacks run on the UI thread as worklets—no
JS thread round-trip for press animations.

### Incorrect (Pressable with JS thread callbacks)

```tsx
import { Pressable } from 'react-native'
import Animated, {
  useSharedValue,
  useAnimatedStyle,
  withTiming,
} from 'react-native-reanimated'

function AnimatedButton({ onPress }: { onPress: () => void }) {
  const scale = useSharedValue(1)

  const animatedStyle = useAnimatedStyle(() => ({
    transform: [{ scale: scale.value }],
  }))

  return (
    <Pressable
      onPress={onPress}
      onPressIn={() => (scale.value = withTiming(0.95))}
      onPressOut={() => (scale.value = withTiming(1))}
    >
      <Animated.View style={animatedStyle}>
        <Text>Press me</Text>
      </Animated.View>
    </Pressable>
  )
}
```

#### Correct (GestureDetector with UI thread worklets)

```tsx
import { Gesture, GestureDetector } from 'react-native-gesture-handler'
import Animated, {
  useSharedValue,
  useAnimatedStyle,
  withTiming,
  interpolate,
  runOnJS,
} from 'react-native-reanimated'

function AnimatedButton({ onPress }: { onPress: () => void }) {
  // Store the press STATE (0 = not pressed, 1 = pressed)
  const pressed = useSharedValue(0)

  const tap = Gesture.Tap()
    .onBegin(() => {
      pressed.set(withTiming(1))
    })
    .onFinalize(() => {
      pressed.set(withTiming(0))
    })
    .onEnd(() => {
      runOnJS(onPress)()
    })

  // Derive visual values from the state
  const animatedStyle = useAnimatedStyle(() => ({
    transform: [
      { scale: interpolate(withTiming(pressed.get()), [0, 1], [1, 0.95]) },
    ],
  }))

  return (
    <GestureDetector gesture={tap}>
      <Animated.View style={animatedStyle}>
        <Text>Press me</Text>
      </Animated.View>
    </GestureDetector>
  )
}
```

Store the press **state** (0 or 1), then derive the scale via `interpolate`.
This keeps the shared value as ground truth. Use `runOnJS` to call JS functions
from worklets. Use `.set()` and `.get()` for React Compiler compatibility.

Reference:
[Gesture Handler Tap Gesture](https://docs.swmansion.com/react-native-gesture-handler/docs/gestures/tap-gesture)

## When to use

Use when the user asks about or needs: Use GestureDetector for Animated Press States.
﻿---
name: Use .get() and .set() for Reanimated Shared Values (not .value)
description: ## Use .get() and .set() for Shared Values with React Compiler
 
 With React Compiler enabled, use `.get()` and `.set()` instead of reading or
tags: reanimated, react-compiler, shared-values
---

## Use .get() and .set() for Shared Values with React Compiler

With React Compiler enabled, use `.get()` and `.set()` instead of reading or
writing `.value` directly on Reanimated shared values. The compiler can't track
property access—explicit methods ensure correct behavior.

### Incorrect (breaks with React Compiler)

```tsx
import { useSharedValue } from 'react-native-reanimated'

function Counter() {
  const count = useSharedValue(0)

  const increment = () => {
    count.value = count.value + 1 // opts out of react compiler
  }

  return <Button onPress={increment} title={`Count: ${count.value}`} />
}
```

#### Correct (React Compiler compatible)

```tsx
import { useSharedValue } from 'react-native-reanimated'

function Counter() {
  const count = useSharedValue(0)

  const increment = () => {
    count.set(count.get() + 1)
  }

  return <Button onPress={increment} title={`Count: ${count.get()}`} />
}
```

See the
[Reanimated docs](https://docs.swmansion.com/react-native-reanimated/docs/core/useSharedValue/#react-compiler-support)
for more.

## When to use

Use when the user asks about or needs: Use .get() and .set() for Reanimated Shared Values (not .value).
﻿---
name: Use contentInsetAdjustmentBehavior for Safe Areas
description: ## Use contentInsetAdjustmentBehavior for Safe Areas
 
 Use `contentInsetAdjustmentBehavior="automatic"` on the root ScrollView instead of wrapping content in SafeAreaView or manual padding. This lets iOS handle safe area insets natively with proper scroll behavior.
tags: safe-area, scrollview, layout
---

## Use contentInsetAdjustmentBehavior for Safe Areas

Use `contentInsetAdjustmentBehavior="automatic"` on the root ScrollView instead of wrapping content in SafeAreaView or manual padding. This lets iOS handle safe area insets natively with proper scroll behavior.

### Incorrect (SafeAreaView wrapper)

```tsx
import { SafeAreaView, ScrollView, View, Text } from 'react-native'

function MyScreen() {
  return (
    <SafeAreaView style={{ flex: 1 }}>
      <ScrollView>
        <View>
          <Text>Content</Text>
        </View>
      </ScrollView>
    </SafeAreaView>
  )
}
```

#### Incorrect (manual safe area padding)

```tsx
import { ScrollView, View, Text } from 'react-native'
import { useSafeAreaInsets } from 'react-native-safe-area-context'

function MyScreen() {
  const insets = useSafeAreaInsets()

  return (
    <ScrollView contentContainerStyle={{ paddingTop: insets.top }}>
      <View>
        <Text>Content</Text>
      </View>
    </ScrollView>
  )
}
```

#### Correct (native content inset adjustment)

```tsx
import { ScrollView, View, Text } from 'react-native'

function MyScreen() {
  return (
    <ScrollView contentInsetAdjustmentBehavior='automatic'>
      <View>
        <Text>Content</Text>
      </View>
    </ScrollView>
  )
}
```

The native approach handles dynamic safe areas (keyboard, toolbars) and allows content to scroll behind the status bar naturally.

## When to use

Use when the user asks about or needs: Use contentInsetAdjustmentBehavior for Safe Areas.
﻿---
name: Use contentInset for Dynamic ScrollView Spacing
description: ## Use contentInset for Dynamic ScrollView Spacing
 
 When adding space to the top or bottom of a ScrollView that may change
tags: scrollview, layout, contentInset, performance
---

## Use contentInset for Dynamic ScrollView Spacing

When adding space to the top or bottom of a ScrollView that may change
(keyboard, toolbars, dynamic content), use `contentInset` instead of padding.
Changing `contentInset` doesn't trigger layout recalculation—it adjusts the
scroll area without re-rendering content.

### Incorrect (padding causes layout recalculation)

```tsx
function Feed({ bottomOffset }: { bottomOffset: number }) {
  return (
    <ScrollView contentContainerStyle={{ paddingBottom: bottomOffset }}>
      {children}
    </ScrollView>
  )
}
// Changing bottomOffset triggers full layout recalculation
```

#### Correct (contentInset for dynamic spacing)

```tsx
function Feed({ bottomOffset }: { bottomOffset: number }) {
  return (
    <ScrollView
      contentInset={{ bottom: bottomOffset }}
      scrollIndicatorInsets={{ bottom: bottomOffset }}
    >
      {children}
    </ScrollView>
  )
}
// Changing bottomOffset only adjusts scroll bounds
```

Use `scrollIndicatorInsets` alongside `contentInset` to keep the scroll
indicator aligned. For static spacing that never changes, padding is fine.

## When to use

Use when the user asks about or needs: Use contentInset for Dynamic ScrollView Spacing.
﻿---
name: Wrap Strings in Text Components
description: ## Wrap Strings in Text Components
 
 Strings must be rendered inside `<Text>`. React Native crashes if a string is a
tags: rendering, text, core
---

## Wrap Strings in Text Components

Strings must be rendered inside `<Text>`. React Native crashes if a string is a
direct child of `<View>`.

### Incorrect (crashes)

```tsx
import { View } from 'react-native'

function Greeting({ name }: { name: string }) {
  return <View>Hello, {name}!</View>
}
// Error: Text strings must be rendered within a <Text> component.
```

#### Correct

```tsx
import { View, Text } from 'react-native'

function Greeting({ name }: { name: string }) {
  return (
    <View>
      <Text>Hello, {name}!</Text>
    </View>
  )
}
```

## When to use

Use when the user asks about or needs: Wrap Strings in Text Components.
﻿---
name: Never Track Scroll Position in useState
description: ## Never Track Scroll Position in useState
 
 Never store scroll position in `useState`. Scroll events fire rapidly—state
tags: scroll, performance, reanimated, useRef
---

## Never Track Scroll Position in useState

Never store scroll position in `useState`. Scroll events fire rapidly—state
updates cause render thrashing and dropped frames. Use a Reanimated shared value
for animations or a ref for non-reactive tracking.

### Incorrect (useState causes jank)

```tsx
import { useState } from 'react'
import {
  ScrollView,
  NativeSyntheticEvent,
  NativeScrollEvent,
} from 'react-native'

function Feed() {
  const [scrollY, setScrollY] = useState(0)

  const onScroll = (e: NativeSyntheticEvent<NativeScrollEvent>) => {
    setScrollY(e.nativeEvent.contentOffset.y) // re-renders on every frame
  }

  return <ScrollView onScroll={onScroll} scrollEventThrottle={16} />
}
```

#### Correct (Reanimated for animations)

```tsx
import Animated, {
  useSharedValue,
  useAnimatedScrollHandler,
} from 'react-native-reanimated'

function Feed() {
  const scrollY = useSharedValue(0)

  const onScroll = useAnimatedScrollHandler({
    onScroll: (e) => {
      scrollY.value = e.contentOffset.y // runs on UI thread, no re-render
    },
  })

  return (
    <Animated.ScrollView
      onScroll={onScroll}
      // higher number has better performance, but it fires less often.
      // unset this if you need higher precision over performance.
      scrollEventThrottle={16}
    />
  )
}
```

#### Correct (ref for non-reactive tracking)

```tsx
import { useRef } from 'react'
import {
  ScrollView,
  NativeSyntheticEvent,
  NativeScrollEvent,
} from 'react-native'

function Feed() {
  const scrollY = useRef(0)

  const onScroll = (e: NativeSyntheticEvent<NativeScrollEvent>) => {
    scrollY.current = e.nativeEvent.contentOffset.y // no re-render
  }

  return <ScrollView onScroll={onScroll} scrollEventThrottle={16} />
}
```

## When to use

Use when the user asks about or needs: Never Track Scroll Position in useState.
﻿---
name: Prefer useDerivedValue Over useAnimatedReaction
description: ## Prefer useDerivedValue Over useAnimatedReaction
 
 When deriving a shared value from another, use `useDerivedValue` instead of
tags: animation, reanimated, derived-value
---

## Prefer useDerivedValue Over useAnimatedReaction

When deriving a shared value from another, use `useDerivedValue` instead of
`useAnimatedReaction`. Derived values are declarative, automatically track
dependencies, and return a value you can use directly. Animated reactions are
for side effects, not derivations.

### Incorrect (useAnimatedReaction for derivation)

```tsx
import { useSharedValue, useAnimatedReaction } from 'react-native-reanimated'

function MyComponent() {
  const progress = useSharedValue(0)
  const opacity = useSharedValue(1)

  useAnimatedReaction(
    () => progress.value,
    (current) => {
      opacity.value = 1 - current
    }
  )

  // ...
}
```

#### Correct (useDerivedValue)

```tsx
import { useSharedValue, useDerivedValue } from 'react-native-reanimated'

function MyComponent() {
  const progress = useSharedValue(0)

  const opacity = useDerivedValue(() => 1 - progress.get())

  // ...
}
```

Use `useAnimatedReaction` only for side effects that don't produce a value
(e.g., triggering haptics, logging, calling `runOnJS`).

Reference:
[Reanimated useDerivedValue](https://docs.swmansion.com/react-native-reanimated/docs/core/useDerivedValue)

## When to use

Use when the user asks about or needs: Prefer useDerivedValue Over useAnimatedReaction.
﻿---
name: Animate SVG Wrapper Instead of SVG Element
description: ## Animate SVG Wrapper Instead of SVG Element
 
 Many browsers don't have hardware acceleration for CSS3 animations on SVG elements. Wrap SVG in a `<div>` and animate the wrapper instead.
tags: rendering, svg, css, animation, performance
---

## Animate SVG Wrapper Instead of SVG Element

Many browsers don't have hardware acceleration for CSS3 animations on SVG elements. Wrap SVG in a `<div>` and animate the wrapper instead.

### Incorrect (animating SVG directly - no hardware acceleration)

```tsx
function LoadingSpinner() {
  return (
    <svg 
      className="animate-spin"
      width="24" 
      height="24" 
      viewBox="0 0 24 24"
    >
      <circle cx="12" cy="12" r="10" stroke="currentColor" />
    </svg>
  )
}
```

#### Correct (animating wrapper div - hardware accelerated)

```tsx
function LoadingSpinner() {
  return (
    <div className="animate-spin">
      <svg 
        width="24" 
        height="24" 
        viewBox="0 0 24 24"
      >
        <circle cx="12" cy="12" r="10" stroke="currentColor" />
      </svg>
    </div>
  )
}
```

This applies to all CSS transforms and transitions (`transform`, `opacity`, `translate`, `scale`, `rotate`). The wrapper div allows browsers to use GPU acceleration for smoother animations.

## When to use

Use when the user asks about or needs: Animate SVG Wrapper Instead of SVG Element.
﻿---
name: Animate Transform and Opacity Instead of Layout Properties
description: ## Animate Transform and Opacity Instead of Layout Properties
 
 Avoid animating `width`, `height`, `top`, `left`, `margin`, or `padding`. These trigger layout recalculation on every frame. Instead, use `transform` (scale, translate) and `opacity` which run on the GPU without triggering layout.
tags: animation, performance, reanimated, transform, opacity
---

## Animate Transform and Opacity Instead of Layout Properties

Avoid animating `width`, `height`, `top`, `left`, `margin`, or `padding`. These trigger layout recalculation on every frame. Instead, use `transform` (scale, translate) and `opacity` which run on the GPU without triggering layout.

### Incorrect (animates height, triggers layout every frame)

```tsx
import Animated, { useAnimatedStyle, withTiming } from 'react-native-reanimated'

function CollapsiblePanel({ expanded }: { expanded: boolean }) {
  const animatedStyle = useAnimatedStyle(() => ({
    height: withTiming(expanded ? 200 : 0), // triggers layout on every frame
    overflow: 'hidden',
  }))

  return <Animated.View style={animatedStyle}>{children}</Animated.View>
}
```

#### Correct (animates scaleY, GPU-accelerated)

```tsx
import Animated, { useAnimatedStyle, withTiming } from 'react-native-reanimated'

function CollapsiblePanel({ expanded }: { expanded: boolean }) {
  const animatedStyle = useAnimatedStyle(() => ({
    transform: [
      { scaleY: withTiming(expanded ? 1 : 0) },
    ],
    opacity: withTiming(expanded ? 1 : 0),
  }))

  return (
    <Animated.View style={[{ height: 200, transformOrigin: 'top' }, animatedStyle]}>
      {children}
    </Animated.View>
  )
}
```

#### Correct (animates translateY for slide animations)

```tsx
import Animated, { useAnimatedStyle, withTiming } from 'react-native-reanimated'

function SlideIn({ visible }: { visible: boolean }) {
  const animatedStyle = useAnimatedStyle(() => ({
    transform: [
      { translateY: withTiming(visible ? 0 : 100) },
    ],
    opacity: withTiming(visible ? 1 : 0),
  }))

  return <Animated.View style={animatedStyle}>{children}</Animated.View>
}
```

GPU-accelerated properties: `transform` (translate, scale, rotate), `opacity`. Everything else triggers layout.

## When to use

Use when the user asks about or needs: Animate Transform and Opacity Instead of Layout Properties.
