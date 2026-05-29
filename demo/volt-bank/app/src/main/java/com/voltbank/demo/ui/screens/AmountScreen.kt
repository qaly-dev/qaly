package com.voltbank.demo.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.voltbank.demo.ui.theme.VoltViolet

@Composable
fun AmountScreen(
    contactName: String,
    contactEmail: String,
    onContinue: (amount: String, note: String) -> Unit,
    onBack: () -> Unit
) {
    var amountDisplay by remember { mutableStateOf("0") }
    var note by remember { mutableStateOf("") }

    val isEnabled = amountDisplay != "0" && amountDisplay.isNotEmpty()

    fun handleKey(key: String) {
        when (key) {
            "backspace" -> {
                if (amountDisplay.length > 1) {
                    amountDisplay = amountDisplay.dropLast(1)
                } else {
                    amountDisplay = "0"
                }
            }
            "." -> {
                if (!amountDisplay.contains(".")) {
                    amountDisplay = "$amountDisplay."
                }
            }
            else -> {
                if (amountDisplay == "0") {
                    amountDisplay = key
                } else {
                    // Max 2 decimal places
                    val dotIndex = amountDisplay.indexOf(".")
                    if (dotIndex == -1 || amountDisplay.length - dotIndex <= 2) {
                        amountDisplay = "$amountDisplay$key"
                    }
                }
            }
        }
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.background)
    ) {
        Spacer(modifier = Modifier.height(52.dp))

        // Top bar
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 8.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            IconButton(
                onClick = onBack,
                modifier = Modifier.semantics { contentDescription = "Go back" }
            ) {
                Icon(
                    imageVector = Icons.Default.ArrowBack,
                    contentDescription = "Go back",
                    tint = MaterialTheme.colorScheme.onBackground
                )
            }
            Text(
                text = "Enter Amount",
                style = MaterialTheme.typography.titleLarge,
                color = MaterialTheme.colorScheme.onBackground,
                modifier = Modifier.padding(start = 8.dp)
            )
        }

        Spacer(modifier = Modifier.height(20.dp))

        // Selected contact
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 20.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.Center
        ) {
            Box(
                modifier = Modifier
                    .size(40.dp)
                    .clip(CircleShape)
                    .background(VoltViolet),
                contentAlignment = Alignment.Center
            ) {
                Text(
                    text = contactName.first().toString(),
                    color = Color.White,
                    fontWeight = FontWeight.Bold,
                    fontSize = 16.sp
                )
            }
            Text(
                text = "  $contactName",
                style = MaterialTheme.typography.titleMedium,
                color = MaterialTheme.colorScheme.onBackground
            )
        }

        Spacer(modifier = Modifier.height(24.dp))

        // Amount display
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 20.dp),
            contentAlignment = Alignment.Center
        ) {
            Text(
                text = "\$$amountDisplay",
                fontSize = 56.sp,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.onBackground
            )
        }

        Spacer(modifier = Modifier.height(24.dp))

        // Note field
        OutlinedTextField(
            value = note,
            onValueChange = { note = it },
            placeholder = {
                Text(
                    text = "Add a note (optional)",
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            },
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 20.dp)
                .semantics { contentDescription = "Add a note" },
            shape = RoundedCornerShape(12.dp),
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = VoltViolet,
                unfocusedBorderColor = MaterialTheme.colorScheme.outline,
                focusedTextColor = MaterialTheme.colorScheme.onSurface,
                unfocusedTextColor = MaterialTheme.colorScheme.onSurface,
                cursorColor = VoltViolet,
                focusedContainerColor = MaterialTheme.colorScheme.surface,
                unfocusedContainerColor = MaterialTheme.colorScheme.surface
            )
        )

        Spacer(modifier = Modifier.height(24.dp))

        // Keypad
        val rows = listOf(
            listOf("1", "2", "3"),
            listOf("4", "5", "6"),
            listOf("7", "8", "9"),
            listOf(".", "0", "backspace")
        )
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 20.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            rows.forEach { row ->
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    row.forEach { key ->
                        val label = if (key == "backspace") "⌫" else key
                        val desc = if (key == "backspace") "Delete" else key
                        TextButton(
                            onClick = { handleKey(key) },
                            modifier = Modifier
                                .weight(1f)
                                .height(56.dp)
                                .semantics {
                                    testTag = "key_${key.replace("backspace", "delete")}"
                                    contentDescription = desc
                                },
                            shape = RoundedCornerShape(12.dp),
                            colors = ButtonDefaults.textButtonColors(
                                containerColor = MaterialTheme.colorScheme.surface,
                                contentColor = MaterialTheme.colorScheme.onSurface
                            )
                        ) {
                            Text(
                                text = label,
                                fontSize = 22.sp,
                                fontWeight = FontWeight.Medium
                            )
                        }
                    }
                }
            }
        }

        Spacer(modifier = Modifier.weight(1f))

        // Continue button
        Button(
            onClick = { onContinue(amountDisplay, note) },
            enabled = isEnabled,
            modifier = Modifier
                .fillMaxWidth()
                .height(56.dp)
                .padding(horizontal = 20.dp)
                .semantics { testTag = "continue_btn"; contentDescription = "Continue" },
            shape = RoundedCornerShape(12.dp),
            colors = ButtonDefaults.buttonColors(
                containerColor = VoltViolet,
                contentColor = Color.White,
                disabledContainerColor = VoltViolet.copy(alpha = 0.4f),
                disabledContentColor = Color.White.copy(alpha = 0.4f)
            )
        ) {
            Text(
                text = "Continue",
                style = MaterialTheme.typography.labelLarge,
                fontSize = 18.sp
            )
        }

        Spacer(modifier = Modifier.height(32.dp))
    }
}
