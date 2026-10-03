package com.hammurabicoding.nocapsnap

import android.os.Bundle
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge

class MainActivity : TauriActivity() {
  private var networkStatus: NetworkStatus? = null

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
  }

  // wry calls this before the first page load, so the page can read the network status as it starts.
  override fun onWebViewCreate(webView: WebView) {
    super.onWebViewCreate(webView)
    networkStatus?.stop()
    networkStatus = NetworkStatus(this, webView).also { it.start() }
  }

  override fun onDestroy() {
    networkStatus?.stop()
    networkStatus = null
    super.onDestroy()
  }
}
