package com.voltbank.demo.navigation

import androidx.compose.runtime.Composable
import androidx.navigation.NavType
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument
import com.voltbank.demo.ui.screens.AmountScreen
import com.voltbank.demo.ui.screens.ConfirmScreen
import com.voltbank.demo.ui.screens.ContactsScreen
import com.voltbank.demo.ui.screens.HomeScreen
import com.voltbank.demo.ui.screens.SuccessScreen

@Composable
fun NavGraph() {
    val navController = rememberNavController()

    NavHost(
        navController = navController,
        startDestination = "home"
    ) {
        composable("home") {
            HomeScreen(
                onSendMoney = { navController.navigate("contacts") }
            )
        }

        composable("contacts") {
            ContactsScreen(
                onContactSelected = { name, email ->
                    navController.navigate("amount/$name/$email")
                },
                onBack = { navController.popBackStack() }
            )
        }

        composable(
            route = "amount/{contactName}/{contactEmail}",
            arguments = listOf(
                navArgument("contactName") { type = NavType.StringType },
                navArgument("contactEmail") { type = NavType.StringType }
            )
        ) { backStackEntry ->
            val contactName = backStackEntry.arguments?.getString("contactName") ?: ""
            val contactEmail = backStackEntry.arguments?.getString("contactEmail") ?: ""
            AmountScreen(
                contactName = contactName,
                contactEmail = contactEmail,
                onContinue = { amount, note ->
                    navController.navigate("confirm/$contactName/$amount/${note.ifEmpty { "none" }}")
                },
                onBack = { navController.popBackStack() }
            )
        }

        composable(
            route = "confirm/{contactName}/{amount}/{note}",
            arguments = listOf(
                navArgument("contactName") { type = NavType.StringType },
                navArgument("amount") { type = NavType.StringType },
                navArgument("note") { type = NavType.StringType }
            )
        ) { backStackEntry ->
            val contactName = backStackEntry.arguments?.getString("contactName") ?: ""
            val amount = backStackEntry.arguments?.getString("amount") ?: "0"
            val note = backStackEntry.arguments?.getString("note") ?: "none"
            ConfirmScreen(
                contactName = contactName,
                amount = amount,
                note = if (note == "none") "" else note,
                onConfirm = {
                    navController.navigate("success/$contactName/$amount") {
                        popUpTo("home")
                    }
                },
                onCancel = {
                    navController.navigate("home") {
                        popUpTo("home") { inclusive = true }
                    }
                }
            )
        }

        composable(
            route = "success/{contactName}/{amount}",
            arguments = listOf(
                navArgument("contactName") { type = NavType.StringType },
                navArgument("amount") { type = NavType.StringType }
            )
        ) { backStackEntry ->
            val contactName = backStackEntry.arguments?.getString("contactName") ?: ""
            val amount = backStackEntry.arguments?.getString("amount") ?: "0"
            SuccessScreen(
                contactName = contactName,
                amount = amount,
                onBackToHome = {
                    navController.navigate("home") {
                        popUpTo("home") { inclusive = true }
                    }
                }
            )
        }
    }
}
