# Volt Bank — qaly Demo App

A purpose-built Android banking demo app used to showcase **qaly's auto-heal capability** in a product demo video.

## What it is

Volt Bank simulates a realistic mobile banking app with 5 screens:

1. **Home** — balance card, recent transactions, and the key CTA button
2. **Contacts** — pick a recipient to send money to
3. **Amount** — enter the transfer amount with a custom keypad
4. **Confirm** — review the transfer before submitting
5. **Success** — confirmation screen with timestamp

## The Demo Switch (`Config.DEMO_VERSION`)

The file `app/src/main/java/com/voltbank/demo/Config.kt` contains a single constant:

```kotlin
object Config {
    const val DEMO_VERSION = 1  // Change to 2 for the "after" state
}
```

| Value | CTA button text | Use case |
|-------|----------------|----------|
| `1`   | `Send Money`   | Record the qaly test |
| `2`   | `Transfer Funds` | Show auto-heal after "rebrand" |

**This is the ONLY thing that changes between v1 and v2.** Everything else — layout, navigation, colors, other text — stays identical.

## How the qaly demo works

1. Set `DEMO_VERSION = 1`, build and install the app
2. Record a qaly test that taps "Send Money"
3. Change `DEMO_VERSION = 2`, rebuild and reinstall
4. Run the qaly test — qaly auto-heals and finds "Transfer Funds" instead of "Send Money"
5. The test passes without any manual intervention

## How to build

### Prerequisites

- Android Studio or command-line Android SDK
- Java 17+
- Android SDK with API 35

### Build debug APK

```bash
cd demo/volt-bank
./gradlew assembleDebug
```

The APK will be at: `app/build/outputs/apk/debug/app-debug.apk`

### Install on emulator/device

```bash
adb install app/build/outputs/apk/debug/app-debug.apk
```

Or use Android Studio's Run button directly.

## Tech stack

- Kotlin 2.0.0
- Jetpack Compose with BOM 2024.06.00
- Material Design 3 (dark theme only)
- Navigation Compose 2.7.7
- AGP 8.4.0 / Gradle 8.8
- compileSdk 35, minSdk 26

## Design

- Background: `#0F1128` (deep navy)
- Cards/surfaces: `#1B1F3B`
- Accent: `#7C3AED` (violet)
- Dark theme only — no light theme variant
