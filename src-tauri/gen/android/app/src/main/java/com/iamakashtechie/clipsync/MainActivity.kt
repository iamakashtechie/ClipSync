package com.iamakashtechie.clipsync

import android.os.Bundle
import androidx.activity.enableEdgeToEdge
import androidx.core.content.ContextCompat
import android.content.Intent

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)

    // Native scaffold: keep a foreground service alive while the app is active.
    val serviceIntent = Intent(this, ClipSyncForegroundService::class.java)
    ContextCompat.startForegroundService(this, serviceIntent)
  }
}
