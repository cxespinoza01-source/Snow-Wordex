package org.wordcraft.android;

import com.google.androidgamesdk.GameActivity;

/**
 * Java entry point for the native Rust/eframe application.
 *
 * GameActivity must be subclassed by the application, and the native
 * library must be loaded before Android calls into its native entry point.
 */
public final class WordCraftActivity extends GameActivity {
    static {
        System.loadLibrary("wordcraft_android");
    }
}
