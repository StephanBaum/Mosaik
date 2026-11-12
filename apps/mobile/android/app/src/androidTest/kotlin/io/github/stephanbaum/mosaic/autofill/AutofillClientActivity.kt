package io.github.stephanbaum.mosaic.autofill

import android.app.Activity
import android.os.Bundle
import android.text.InputType
import android.view.View
import android.widget.EditText
import android.widget.LinearLayout

/** A plain client app with a user name and a password field, for the Autofill tests. */
class AutofillClientActivity : Activity() {
    lateinit var username: EditText
        private set
    lateinit var password: EditText
        private set

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        username =
            EditText(this).apply {
                setAutofillHints(View.AUTOFILL_HINT_USERNAME)
                importantForAutofill = View.IMPORTANT_FOR_AUTOFILL_YES
                inputType = InputType.TYPE_CLASS_TEXT
            }
        password =
            EditText(this).apply {
                setAutofillHints(View.AUTOFILL_HINT_PASSWORD)
                importantForAutofill = View.IMPORTANT_FOR_AUTOFILL_YES
                inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_PASSWORD
            }
        val column = LinearLayout(this)
        column.orientation = LinearLayout.VERTICAL
        column.addView(username)
        column.addView(password)
        setContentView(column)
    }
}
