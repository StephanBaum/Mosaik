package io.github.stephanbaum.mosaic.autofill

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.text.InputType
import android.view.View
import android.widget.EditText
import android.widget.LinearLayout

/**
 * A plain client app with a user name and a password field, for the Autofill tests. It runs in the process of the
 * test APK, so the tests find its fields through the accessibility tree (the content descriptions name them).
 * Both fields are plain text so that the tree shows what was filled.
 */
class AutofillClientActivity : Activity() {
    private lateinit var username: EditText
    private lateinit var password: EditText

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        username = field(View.AUTOFILL_HINT_USERNAME, "client-username")
        password = field(View.AUTOFILL_HINT_PASSWORD, "client-password")
        val column = LinearLayout(this)
        column.orientation = LinearLayout.VERTICAL
        column.addView(username)
        column.addView(password)
        setContentView(column)
    }

    /** A new start brings the single instance forward: it starts empty again. */
    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        username.setText("")
        password.setText("")
    }

    private fun field(hint: String, description: String): EditText =
        EditText(this).apply {
            setAutofillHints(hint)
            importantForAutofill = View.IMPORTANT_FOR_AUTOFILL_YES
            contentDescription = description
            inputType = InputType.TYPE_CLASS_TEXT
        }
}
