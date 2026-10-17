package com.sc.vulkanodemo

import android.os.Bundle
import android.view.WindowManager
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import com.google.androidgamesdk.GameActivity

class MainActivity : GameActivity() {
    private fun hideSystemUI() {
        // This will put the game behind any cutouts and waterfalls on devices which have
        // them, so the corresponding insets will be non-zero.
        window.attributes.layoutInDisplayCutoutMode =
            WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_ALWAYS
        // From API 30 onwards, this is the recommended way to hide the system UI, rather than
        // using View.setSystemUiVisibility.
        val controller = WindowInsetsControllerCompat(window, window.decorView)
        controller.hide(WindowInsetsCompat.Type.systemBars())
        controller.hide(WindowInsetsCompat.Type.displayCutout())
        controller.systemBarsBehavior =
            WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        // When true, the app will fit inside any system UI windows.
        // When false, we render behind any system UI windows.
        WindowCompat.setDecorFitsSystemWindows(window, false)
        hideSystemUI()
        // You can set IME fields here or in native code using GameActivity_setImeEditorInfoFields.
        // We set the fields in native_engine.cpp.
        // super.setImeEditorInfoFields(InputType.TYPE_CLASS_TEXT,
        //     IME_ACTION_NONE, IME_FLAG_NO_FULLSCREEN );
        super.onCreate(savedInstanceState)
    }

    override fun onResume() {
        super.onResume()
        hideSystemUI()
    }

    companion object {
        init {
            // Load the native library.
            System.loadLibrary("main")
        }
    }

    fun listAssetFiles(): List<String> {
        val list = ArrayList<String>()
        listAssetFilesRecursive("", list)
        return list
    }

    private fun listAssetFilesRecursive(path: String, outList: MutableList<String>) {
        assets.list(path)?.let {
            if (it.isEmpty()) { // path is a file
                outList.add(path)
            } else { // path is a directory
                for (entry in it) {
                    listAssetFilesRecursive(if (path.isEmpty()) entry else "$path/$entry", outList)
                }
            }
        }
    }
}
