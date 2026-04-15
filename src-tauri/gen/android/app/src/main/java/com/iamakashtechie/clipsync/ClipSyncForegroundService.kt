package com.iamakashtechie.clipsync

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Intent
import android.os.Build
import android.os.IBinder

class ClipSyncForegroundService : Service() {
  override fun onCreate() {
    super.onCreate()
    startForegroundInternal()
  }

  override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
    startForegroundInternal()
    return START_STICKY
  }

  override fun onBind(intent: Intent?): IBinder? {
    return null
  }

  private fun startForegroundInternal() {
    val channelId = "clipsync_bg_channel"

    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
      val channel = NotificationChannel(
        channelId,
        "ClipSync Background",
        NotificationManager.IMPORTANCE_LOW,
      )
      channel.description = "Keeps ClipSync transport active in background"

      val manager = getSystemService(NotificationManager::class.java)
      manager?.createNotificationChannel(channel)
    }

    val notification = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
      Notification.Builder(this, channelId)
        .setContentTitle("ClipSync")
        .setContentText("Background reliability mode active")
        .setSmallIcon(android.R.drawable.stat_notify_sync)
        .setOngoing(true)
        .build()
    } else {
      Notification.Builder(this)
        .setContentTitle("ClipSync")
        .setContentText("Background reliability mode active")
        .setSmallIcon(android.R.drawable.stat_notify_sync)
        .setOngoing(true)
        .build()
    }

    startForeground(1001, notification)
  }
}
