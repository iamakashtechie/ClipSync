package com.iamakashtechie.clipsync

import android.Manifest
import android.content.BroadcastReceiver
import android.content.Context
import android.os.Bundle
import android.content.IntentFilter
import android.content.pm.PackageManager
import android.webkit.WebView
import androidx.core.app.ActivityCompat
import androidx.activity.enableEdgeToEdge
import androidx.core.content.ContextCompat
import android.content.Intent
import org.json.JSONObject

class MainActivity : TauriActivity() {
  private val notificationPermissionReqCode = 1019
  private var webViewRef: WebView? = null

  private val clipboardReceiver = object : BroadcastReceiver() {
    override fun onReceive(context: Context?, intent: Intent?) {
      if (intent?.action != CLIPSYNC_ACTION_NATIVE_CLIPBOARD_CHANGED) {
        return
      }

      val text = intent.getStringExtra(CLIPSYNC_EXTRA_TEXT) ?: return
      val source = intent.getStringExtra(CLIPSYNC_EXTRA_SOURCE) ?: "native"
      dispatchClipboardToWebView(text, source)
    }
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    ensureForegroundServiceStarted()
  }

  private fun ensureForegroundServiceStarted() {
    if (
      android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.TIRAMISU &&
      ContextCompat.checkSelfPermission(this, Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
    ) {
      ActivityCompat.requestPermissions(
        this,
        arrayOf(Manifest.permission.POST_NOTIFICATIONS),
        notificationPermissionReqCode,
      )
      return
    }

    // Keep foreground service alive for best-effort background clipboard capture.
    val serviceIntent = Intent(this, ClipSyncForegroundService::class.java)
    ContextCompat.startForegroundService(this, serviceIntent)
  }

  override fun onRequestPermissionsResult(
    requestCode: Int,
    permissions: Array<out String>,
    grantResults: IntArray,
  ) {
    super.onRequestPermissionsResult(requestCode, permissions, grantResults)
    if (requestCode == notificationPermissionReqCode) {
      ensureForegroundServiceStarted()
    }
  }

  override fun onStart() {
    super.onStart()
    ContextCompat.registerReceiver(
      this,
      clipboardReceiver,
      IntentFilter(CLIPSYNC_ACTION_NATIVE_CLIPBOARD_CHANGED),
      ContextCompat.RECEIVER_NOT_EXPORTED,
    )
  }

  override fun onStop() {
    runCatching {
      unregisterReceiver(clipboardReceiver)
    }
    super.onStop()
  }

  override fun onResume() {
    super.onResume()
    consumePendingNativeClipboard(this)?.let { (text, source) ->
      dispatchClipboardToWebView(text, source)
    }
  }

  override fun onWebViewCreate(webView: WebView) {
    super.onWebViewCreate(webView)
    webViewRef = webView
    consumePendingNativeClipboard(this)?.let { (text, source) ->
      dispatchClipboardToWebView(text, source)
    }
  }

  private fun dispatchClipboardToWebView(text: String, source: String) {
    val webView = webViewRef ?: return
    val js = "window.dispatchEvent(new CustomEvent('clipsync-native-clipboard', { detail: { text: ${JSONObject.quote(text)}, source: ${JSONObject.quote(source)}, timestampMs: Date.now() } }));"
    webView.evaluateJavascript(js, null)
  }
}
