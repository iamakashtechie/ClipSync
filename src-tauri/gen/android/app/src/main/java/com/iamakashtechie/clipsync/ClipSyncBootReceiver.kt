package com.iamakashtechie.clipsync

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import androidx.core.content.ContextCompat

class ClipSyncBootReceiver : BroadcastReceiver() {
  override fun onReceive(context: Context, intent: Intent?) {
    val action = intent?.action ?: return
    if (action != Intent.ACTION_BOOT_COMPLETED && action != Intent.ACTION_LOCKED_BOOT_COMPLETED) {
      return
    }

    val prefs = context.getSharedPreferences(CLIPSYNC_NATIVE_PREFS, Context.MODE_PRIVATE)
    val backgroundModeEnabled = prefs.getBoolean("background_mode_enabled", true)

    if (!backgroundModeEnabled) {
      publishNativeRuntimeEvent(
        context,
        "INFO",
        "Boot receiver skipped foreground service start because background reliability mode is disabled.",
        "boot_receiver",
      )
      return
    }

    if (action == Intent.ACTION_LOCKED_BOOT_COMPLETED) {
      publishNativeRuntimeEvent(
        context,
        "INFO",
        "Locked boot received; waiting for full boot completion before starting ClipSync foreground service.",
        "boot_receiver",
      )
      return
    }

    runCatching {
      val serviceIntent = Intent(context, ClipSyncForegroundService::class.java)
      ContextCompat.startForegroundService(context, serviceIntent)
      publishNativeRuntimeEvent(
        context,
        "SUCCESS",
        "Boot receiver started ClipSync foreground service after boot completion.",
        "boot_receiver",
      )
    }.onFailure { error ->
      publishNativeRuntimeEvent(
        context,
        "FAILED",
        "Boot receiver failed to start foreground service: ${error.message ?: "unknown error"}",
        "boot_receiver",
      )
    }
  }
}
