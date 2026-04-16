package com.iamakashtechie.clipsync

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.ClipboardManager
import android.content.Context
import android.app.Service
import android.content.Intent
import android.os.Build
import android.os.IBinder

class ClipSyncForegroundService : Service(), ClipboardManager.OnPrimaryClipChangedListener {
  private var clipboardManager: ClipboardManager? = null

  override fun onCreate() {
    super.onCreate()
    startForegroundInternal()
    clipboardManager = getSystemService(Context.CLIPBOARD_SERVICE) as? ClipboardManager
    clipboardManager?.addPrimaryClipChangedListener(this)
    publishCurrentClipboard("foreground_service_start")
  }

  override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
    startForegroundInternal()
    publishCurrentClipboard("foreground_service_resume")
    return START_STICKY
  }

  override fun onBind(intent: Intent?): IBinder? {
    return null
  }

  override fun onDestroy() {
    clipboardManager?.removePrimaryClipChangedListener(this)
    clipboardManager = null
    super.onDestroy()
  }

  override fun onPrimaryClipChanged() {
    publishCurrentClipboard("foreground_service")
  }

  private fun publishCurrentClipboard(source: String) {
    val clip = clipboardManager?.primaryClip ?: return
    if (clip.itemCount <= 0) {
      return
    }

    val text = clip.getItemAt(0).coerceToText(this)?.toString() ?: return
    publishNativeClipboardText(this, text, source)
  }

  private fun startForegroundInternal() {
    val channelId = "clipsync_bg_channel"

    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
      val channel = NotificationChannel(
        channelId,
        "ClipSync Background",
        NotificationManager.IMPORTANCE_DEFAULT,
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
        .setCategory(Notification.CATEGORY_SERVICE)
        .setOngoing(true)
        .build()
    } else {
      Notification.Builder(this)
        .setContentTitle("ClipSync")
        .setContentText("Background reliability mode active")
        .setSmallIcon(android.R.drawable.stat_notify_sync)
        .setCategory(Notification.CATEGORY_SERVICE)
        .setOngoing(true)
        .build()
    }

    startForeground(1001, notification)
  }
}
