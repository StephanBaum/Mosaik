package io.github.stephanbaum.mosaic.ui.vault

import android.content.res.Resources
import io.github.stephanbaum.mosaic.ffi.FieldView
import io.github.stephanbaum.mosaic.ffi.ItemKind
import io.github.stephanbaum.mosaic.ui.l10n.L10n

/** The label of a field of the generic detail: a custom field brings its own, a built-in one has a stable key. */
fun fieldLabel(r: Resources, field: FieldView): String {
    val custom = field.label
    if (custom != null) return custom
    return when (field.key) {
        "notes" -> L10n.itemFieldNotes(r)
        "login.username" -> L10n.itemFieldUsername(r)
        "login.password" -> L10n.itemFieldPassword(r)
        "login.change_password_url" -> L10n.itemFieldChangePasswordUrl(r)
        "card.cardholder" -> L10n.itemFieldCardholderName(r)
        "card.brand" -> L10n.itemFieldCardBrand(r)
        "card.expiry" -> L10n.itemFieldCardExpiry(r)
        "card.number" -> L10n.itemFieldCardNumber(r)
        "card.cvv" -> L10n.itemFieldCardVerificationCode(r)
        "card.pin" -> L10n.itemFieldCardPin(r)
        "identity.first_name" -> L10n.itemFieldFirstName(r)
        "identity.middle_name" -> L10n.itemFieldMiddleName(r)
        "identity.last_name" -> L10n.itemFieldLastName(r)
        "identity.email" -> L10n.itemFieldEmail(r)
        "identity.phone" -> L10n.itemFieldPhone(r)
        "identity.company" -> L10n.itemFieldCompany(r)
        "identity.address1" -> L10n.itemFieldAddressLine1(r)
        "identity.address2" -> L10n.itemFieldAddressLine2(r)
        "identity.city" -> L10n.itemFieldCity(r)
        "identity.region" -> L10n.itemFieldRegion(r)
        "identity.postal_code" -> L10n.itemFieldPostalCode(r)
        "identity.country" -> L10n.itemFieldCountry(r)
        "identity.birth_date" -> L10n.itemFieldBirthDate(r)
        "identity.passport_number" -> L10n.itemFieldPassportNumber(r)
        "identity.license_number" -> L10n.itemFieldLicenseNumber(r)
        "identity.national_id" -> L10n.itemFieldNationalId(r)
        "api_key.key_id" -> L10n.itemFieldApiKeyId(r)
        "api_key.host" -> L10n.itemFieldApiKeyHost(r)
        "api_key.secret" -> L10n.itemFieldApiKeySecret(r)
        "wifi.ssid" -> L10n.itemFieldWifiSsid(r)
        "wifi.security" -> L10n.itemFieldWifiSecurity(r)
        "wifi.password" -> L10n.itemFieldWifiPassword(r)
        "ssh.comment" -> L10n.itemFieldSshComment(r)
        "ssh.passphrase" -> L10n.itemFieldSshPassphrase(r)
        "ssh.public_key" -> L10n.itemFieldSshPublicKey(r)
        "ssh.private_key" -> L10n.itemFieldSshPrivateKey(r)
        "ssh.fingerprint" -> L10n.itemFieldSshFingerprint(r)
        else -> field.key
    }
}

/** The name of an item kind. */
fun kindLabel(r: Resources, kind: ItemKind): String =
    when (kind) {
        ItemKind.LOGIN -> L10n.itemKindLogin(r)
        ItemKind.PASSKEY -> L10n.itemKindPasskey(r)
        ItemKind.TOTP -> L10n.itemKindTotp(r)
        ItemKind.SECURE_NOTE -> L10n.itemKindSecureNote(r)
        ItemKind.CREDIT_CARD -> L10n.itemKindCreditCard(r)
        ItemKind.IDENTITY -> L10n.itemKindIdentity(r)
        ItemKind.SSH_KEY -> L10n.itemKindSshKey(r)
        ItemKind.API_KEY -> L10n.itemKindApiKey(r)
        ItemKind.WIFI -> L10n.itemKindWifi(r)
        ItemKind.DOCUMENT -> L10n.itemKindDocument(r)
        ItemKind.OTHER -> L10n.itemKindUnsupported(r)
    }

/** The text of an app URL (`androidapp://pkg?cert=...`) shown as the package name. */
fun appUrlName(url: String): String = url.removePrefix("androidapp://").substringBefore('?')
