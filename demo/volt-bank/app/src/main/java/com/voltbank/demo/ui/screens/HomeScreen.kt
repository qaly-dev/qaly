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
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Notifications
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Divider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
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
import com.voltbank.demo.Config
import com.voltbank.demo.ui.theme.VoltDivider
import com.voltbank.demo.ui.theme.VoltNegative
import com.voltbank.demo.ui.theme.VoltPositive
import com.voltbank.demo.ui.theme.VoltTextSecondary
import com.voltbank.demo.ui.theme.VoltViolet

data class Transaction(
    val title: String,
    val subtitle: String,
    val amount: String,
    val isPositive: Boolean
)

private val recentTransactions = listOf(
    Transaction("Netflix", "Subscription", "-\$15.99", false),
    Transaction("Alice Johnson", "Transfer", "+\$250.00", true),
    Transaction("Starbucks", "Purchase", "-\$6.50", false)
)

@Composable
fun HomeScreen(onSendMoney: () -> Unit) {
    val isV2 = Config.DEMO_VERSION != 1
    // V1: "Send Money"     inline (above transactions), testTag = "send_money_btn"
    // V2: "Transfer Funds" below transactions,          testTag = "send_funds_btn"
    //     → @send_money_btn selector fails; edit-distance 4 → auto-heal finds @send_funds_btn
    val ctaLabel = if (!isV2) "Send Money" else "Transfer Funds"
    val ctaTag   = if (!isV2) "send_money_btn" else "send_funds_btn"

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.background)
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 20.dp)
    ) {
        Spacer(modifier = Modifier.height(52.dp))

        // Top bar
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                text = "⚡ Volt Bank",
                style = MaterialTheme.typography.titleLarge,
                color = VoltViolet,
                fontWeight = FontWeight.Bold,
                fontSize = 22.sp
            )
            IconButton(
                onClick = {},
                modifier = Modifier.semantics { contentDescription = "Notifications" }
            ) {
                Icon(
                    imageVector = Icons.Default.Notifications,
                    contentDescription = "Notifications",
                    tint = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }

        Spacer(modifier = Modifier.height(24.dp))

        // Greeting
        Text(
            text = "Good morning, Sarah 👋",
            style = MaterialTheme.typography.headlineMedium,
            color = MaterialTheme.colorScheme.onBackground
        )

        Spacer(modifier = Modifier.height(24.dp))

        // Balance card
        Card(
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(16.dp),
            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
            border = androidx.compose.foundation.BorderStroke(1.dp, VoltViolet)
        ) {
            Column(
                modifier = Modifier.padding(24.dp)
            ) {
                Text(
                    text = "Available Balance",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
                Spacer(modifier = Modifier.height(8.dp))
                Text(
                    text = "\$12,845.50",
                    style = MaterialTheme.typography.displayLarge,
                    color = MaterialTheme.colorScheme.onSurface,
                    fontWeight = FontWeight.Bold
                )
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = "USD · **** 4821",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }

        // V1: CTA button between balance card and transactions
        if (!isV2) {
            Spacer(modifier = Modifier.height(20.dp))
            Button(
                onClick = onSendMoney,
                modifier = Modifier
                    .fillMaxWidth()
                    .height(56.dp)
                    .semantics { testTag = ctaTag; contentDescription = ctaLabel },
                shape = RoundedCornerShape(12.dp),
                colors = ButtonDefaults.buttonColors(
                    containerColor = VoltViolet,
                    contentColor = Color.White
                )
            ) {
                Text(
                    text = ctaLabel,
                    style = MaterialTheme.typography.labelLarge,
                    fontSize = 18.sp
                )
            }
        }

        Spacer(modifier = Modifier.height(32.dp))

        // Recent Transactions
        Text(
            text = "Recent Transactions",
            style = MaterialTheme.typography.titleMedium,
            color = MaterialTheme.colorScheme.onBackground,
            fontWeight = FontWeight.SemiBold
        )

        Spacer(modifier = Modifier.height(12.dp))

        Card(
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(16.dp),
            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)
        ) {
            Column {
                recentTransactions.forEachIndexed { index, transaction ->
                    TransactionRow(transaction = transaction)
                    if (index < recentTransactions.size - 1) {
                        Divider(
                            modifier = Modifier.padding(horizontal = 16.dp),
                            color = VoltDivider,
                            thickness = 0.5.dp
                        )
                    }
                }
            }
        }

        // V2: CTA button below the transactions list
        if (isV2) {
            Spacer(modifier = Modifier.height(20.dp))
            Button(
                onClick = onSendMoney,
                modifier = Modifier
                    .fillMaxWidth()
                    .height(56.dp)
                    .semantics { testTag = ctaTag; contentDescription = ctaLabel },
                shape = RoundedCornerShape(12.dp),
                colors = ButtonDefaults.buttonColors(
                    containerColor = VoltViolet,
                    contentColor = Color.White
                )
            ) {
                Text(
                    text = ctaLabel,
                    style = MaterialTheme.typography.labelLarge,
                    fontSize = 18.sp
                )
            }
        }

        Spacer(modifier = Modifier.height(32.dp))
    }
}

@Composable
private fun TransactionRow(transaction: Transaction) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp, vertical = 14.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            Box(
                modifier = Modifier
                    .size(44.dp)
                    .clip(CircleShape)
                    .background(VoltViolet.copy(alpha = 0.2f)),
                contentAlignment = Alignment.Center
            ) {
                Text(
                    text = transaction.title.first().toString(),
                    color = VoltViolet,
                    fontWeight = FontWeight.Bold,
                    fontSize = 18.sp
                )
            }
            Column {
                Text(
                    text = transaction.title,
                    style = MaterialTheme.typography.bodyLarge,
                    color = MaterialTheme.colorScheme.onSurface,
                    fontWeight = FontWeight.Medium
                )
                Text(
                    text = transaction.subtitle,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    fontSize = 13.sp
                )
            }
        }
        Text(
            text = transaction.amount,
            style = MaterialTheme.typography.bodyLarge,
            color = if (transaction.isPositive) VoltPositive else VoltNegative,
            fontWeight = FontWeight.SemiBold
        )
    }
}
