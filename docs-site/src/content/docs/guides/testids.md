---
title: Adding testIDs for stable selectors
description: How to add testIDs to your app so Qaly can find elements reliably.
---

## Why testIDs?

Qaly can find elements by label without testIDs. But labels can change. `testID` / `resource-id` is a stable contract between your app and your tests.

## Native Android (XML layouts)

```xml
<Button
    android:id="@+id/btn_checkout"
    android:text="Checkout" />
```

Qaly selector: `@btn_checkout`

## Jetpack Compose

```kotlin
Button(
    modifier = Modifier.testTag("checkout_button"),
    onClick = { }
) {
    Text("Checkout")
}
```

Qaly selector: `@checkout_button`

## React Native

```jsx
<TouchableOpacity testID="checkout_button">
  <Text>Checkout</Text>
</TouchableOpacity>
```

Qaly selector: `@checkout_button`

## Flutter (3.13+)

```dart
Semantics(
  identifier: 'checkout_button',
  child: ElevatedButton(
    onPressed: () {},
    child: Text('Checkout'),
  ),
)
```

Qaly selector: `@checkout_button`
