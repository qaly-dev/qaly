package com.voltbank.demo

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.Box
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTagsAsResourceId
import com.voltbank.demo.navigation.NavGraph
import com.voltbank.demo.ui.theme.VoltBankTheme

class MainActivity : ComponentActivity() {
    @OptIn(ExperimentalComposeUiApi::class)
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            VoltBankTheme {
                // testTagsAsResourceId exposes Modifier.testTag() values as
                // UIAutomator resource-ids so qaly can resolve @send_money_btn etc.
                Box(modifier = Modifier.semantics { testTagsAsResourceId = true }) {
                    NavGraph()
                }
            }
        }
    }
}
