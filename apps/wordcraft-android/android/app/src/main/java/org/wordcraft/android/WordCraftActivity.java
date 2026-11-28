package org.wordcraft.android;

import android.database.Cursor;
import android.net.Uri;
import android.os.Bundle;
import android.provider.OpenableColumns;

import androidx.activity.result.ActivityResultLauncher;
import androidx.activity.result.contract.ActivityResultContracts;

import com.google.androidgamesdk.GameActivity;

import java.io.File;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.io.OutputStream;

/**
 * Java entry point for the native Rust/eframe application.
 *
 * The Storage Access Framework runs in Android's Activity lifecycle. The selected
 * content URI is copied into the app cache, then its ordinary path is returned to Rust.
 */
public final class WordCraftActivity extends GameActivity {
    private ActivityResultLauncher<String[]> documentPicker;
    private ActivityResultLauncher<String> saveDocumentPicker;
    private Uri currentSaveUri;
    private String pendingSaveName = "Document.docx";

    static {
        System.loadLibrary("wordcraft_android");
    }

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        documentPicker = registerForActivityResult(
                new ActivityResultContracts.OpenDocument(),
                this::onDocumentPicked
        );
        saveDocumentPicker = registerForActivityResult(
                new ActivityResultContracts.CreateDocument("application/octet-stream"),
                this::onSaveDocumentPicked
        );
    }

    /** Called from Rust through the Activity reference supplied by AndroidApp. */
    public void openDocumentPicker() {
        runOnUiThread(() -> {
            if (documentPicker == null) {
                nativeOnDocumentPicked("", "Android document picker is not ready");
                return;
            }
            try {
                documentPicker.launch(new String[]{"*/*"});
            } catch (RuntimeException error) {
                nativeOnDocumentPicked("", error.getMessage());
            }
        });
    }

    /** Launch Android's system dialog to create a document at a provider-selected location. */
    public void createDocument(String suggestedName) {
        runOnUiThread(() -> {
            if (saveDocumentPicker == null) {
                nativeOnDocumentPicked("", "Android save picker is not ready");
                return;
            }
            String safeName = suggestedName == null ? "Document.docx" : suggestedName;
            safeName = safeName.replaceAll("[\\\\/\\p{Cntrl}]", "_");
            if (safeName.trim().isEmpty() || ".".equals(safeName) || "..".equals(safeName)) {
                safeName = "Document.docx";
            }
            pendingSaveName = safeName;
            try {
                saveDocumentPicker.launch(pendingSaveName);
            } catch (RuntimeException error) {
                nativeOnDocumentPicked("", error.getMessage());
            }
        });
    }

    private void onSaveDocumentPicked(Uri uri) {
        if (uri == null) {
            nativeOnDocumentPicked("", null);
            return;
        }
        currentSaveUri = uri;
        String displayName = pendingSaveName;
        try (Cursor cursor = getContentResolver().query(
                uri, new String[]{OpenableColumns.DISPLAY_NAME}, null, null, null)) {
            if (cursor != null && cursor.moveToFirst()) {
                int nameColumn = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME);
                if (nameColumn >= 0) {
                    String value = cursor.getString(nameColumn);
                    if (value != null && !value.trim().isEmpty()) {
                        displayName = value;
                    }
                }
            }
        } catch (RuntimeException ignored) {
            // The suggested name is still a valid logical name if a provider omits metadata.
        }
        nativeOnDocumentPicked(displayName, null);
    }

    /** Write bytes produced by WordCraft to the URI most recently selected above. */
    public void writeDocument(byte[] bytes) throws Exception {
        Uri target = currentSaveUri;
        if (target == null) {
            throw new IllegalStateException("Choose a save location before saving this document");
        }
        try (OutputStream output = getContentResolver().openOutputStream(target, "wt")) {
            if (output == null) {
                throw new IllegalStateException("Android could not open the selected document for writing");
            }
            output.write(bytes);
            output.flush();
        }
    }

    private void onDocumentPicked(Uri uri) {
        if (uri == null) {
            nativeOnDocumentPicked("", null);
            return;
        }

        try {
            File staged = copyToCache(uri);
            nativeOnDocumentPicked(staged.getAbsolutePath(), null);
        } catch (Exception error) {
            nativeOnDocumentPicked("", error.getMessage());
        }
    }

    private File copyToCache(Uri uri) throws Exception {
        String displayName = "document";
        try (Cursor cursor = getContentResolver().query(
                uri,
                new String[]{OpenableColumns.DISPLAY_NAME},
                null,
                null,
                null
        )) {
            if (cursor != null && cursor.moveToFirst()) {
                int nameColumn = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME);
                if (nameColumn >= 0) {
                    String value = cursor.getString(nameColumn);
                    if (value != null && !value.trim().isEmpty()) {
                        displayName = value;
                    }
                }
            }
        }

        // A provider-supplied display name must never be allowed to escape this directory.
        displayName = displayName.replaceAll("[\\\\/\\p{Cntrl}]", "_");
        if (displayName.trim().isEmpty() || ".".equals(displayName) || "..".equals(displayName)) {
            displayName = "document";
        }

        File directory = new File(getCacheDir(), "wordcraft-open");
        if (!directory.exists() && !directory.mkdirs()) {
            throw new IllegalStateException("Could not create WordCraft's document cache");
        }

        File destination = new File(directory, System.currentTimeMillis() + "-" + displayName);
        try (InputStream input = getContentResolver().openInputStream(uri);
             OutputStream output = new FileOutputStream(destination)) {
            if (input == null) {
                throw new IllegalStateException("Android could not read the selected document");
            }
            byte[] buffer = new byte[64 * 1024];
            int count;
            while ((count = input.read(buffer)) != -1) {
                output.write(buffer, 0, count);
            }
        } catch (Exception error) {
            // Do not leave a partial file behind if the provider fails during the copy.
            //noinspection ResultOfMethodCallIgnored
            destination.delete();
            throw error;
        }
        return destination;
    }

    private static native void nativeOnDocumentPicked(String path, String error);
}
