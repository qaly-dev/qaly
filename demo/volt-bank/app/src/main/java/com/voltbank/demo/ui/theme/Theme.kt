package com.voltbank.demo.ui.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable

private val VoltDarkColorScheme = darkColorScheme(
    primary = VoltViolet,
    onPrimary = VoltTextPrimary,
    primaryContainer = VoltVioletDark,
    onPrimaryContainer = VoltTextPrimary,
    secondary = VoltVioletLight,
    onSecondary = VoltTextPrimary,
    background = VoltNavy,
    onBackground = VoltTextPrimary,
    surface = VoltSurface,
    onSurface = VoltTextPrimary,
    onSurfaceVariant = VoltTextSecondary,
    outline = VoltDivider,
    surfaceVariant = VoltSurface,
)

@Composable
fun VoltBankTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = VoltDarkColorScheme,
        typography = VoltTypography,
        content = content
    )
}
