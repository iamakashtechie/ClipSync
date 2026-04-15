package com.iamakashtechie.clipsync

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import androidx.core.content.ContextCompat

class ClipSyncBootReceiver : BroadcastReceiver() {
  override fun onReceive(context: Context, intent: Intent?) {
    if (intent?.action == Intent.ACTION_BOOT_COMPLETED || intent?.action == Intent.ACTION_LOCKED_BOOT_COMPLETED) {
      val serviceIntent = Intent(context, ClipSyncForegroundService::class.java)
      ContextCompat.startForegroundService(context, serviceIntent)
    }
  }
}
