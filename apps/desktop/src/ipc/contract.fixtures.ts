// Generated from the Rust contract (apps/desktop/src-tauri/src/contract) by
// `MOSAIC_UPDATE_FIXTURES=1 cargo test -p mosaic-desktop contract_fixtures`.
// Do not edit. `tsc` checks it against the TypeScript contract (ADR-0018).
import type { ContractFixtures } from "./contract/fixtures.ts";

export const fixtures = {
  "commands": [
    "session_state",
    "create_account",
    "verify_emergency_kit",
    "unlock",
    "unlock_quick",
    "lock",
    "subscribe_events",
    "list_vaults",
    "create_vault",
    "rename_vault",
    "list_items",
    "get_item",
    "reveal_secret",
    "add_item",
    "update_item",
    "set_favorite",
    "trash_item",
    "restore_item",
    "purge_item",
    "move_item",
    "copy_to_clipboard",
    "clear_clipboard",
    "generate_password",
    "totp_code",
    "get_settings",
    "update_settings",
    "security_report",
    "import_choose_file",
    "import_preview",
    "import_commit",
    "import_cancel",
    "ssh_agent_status",
    "ssh_agent_set_enabled",
    "ssh_respond"
  ],
  "enums": {
    "customFieldKind": [
      "text",
      "hidden",
      "url",
      "email",
      "phone",
      "date",
      "otp"
    ],
    "errorCode": [
      "notImplemented",
      "noAccount",
      "accountExists",
      "locked",
      "alreadyUnlocked",
      "wrongCredentials",
      "secretKeyRequired",
      "secretKeyMalformed",
      "secretKeyMistyped",
      "emptyPassword",
      "quickUnlockUnavailable",
      "storeInUse",
      "storeUnavailable",
      "vaultNotFound",
      "vaultUnavailable",
      "itemNotFound",
      "itemDeleted",
      "sameVault",
      "noOtp",
      "fieldEmpty",
      "invalidInput",
      "reauthRequired",
      "attachmentTooLarge",
      "attachmentQuota",
      "clipboardUnavailable",
      "importFileUnreadable",
      "importFormatInvalid",
      "importPreviewExpired",
      "sshRequestExpired",
      "sshAgentUnavailable",
      "busy",
      "internal"
    ],
    "importFormat": [
      "enpass",
      "bitwarden",
      "keepass",
      "onePassword",
      "csv"
    ],
    "importWarning": [
      "unmappedFields",
      "attachmentsSkipped",
      "entriesSkipped"
    ],
    "insightAction": [
      "changePassword",
      "setUpTotp",
      "reviewUrl"
    ],
    "insightKind": [
      "weak",
      "reused",
      "old",
      "missingTotp",
      "insecureUrl"
    ],
    "insightSeverity": [
      "low",
      "medium",
      "high"
    ],
    "itemKind": [
      "login",
      "passkey",
      "totp",
      "secureNote",
      "creditCard",
      "identity",
      "sshKey",
      "apiKey",
      "wifi",
      "document",
      "unsupported"
    ],
    "language": [
      "system",
      "en",
      "de"
    ],
    "lockReason": [
      "manual",
      "idle",
      "sleep",
      "screenLock"
    ],
    "otpScheme": [
      "totp",
      "hotp",
      "steam"
    ],
    "quickUnlockKind": [
      "biometric",
      "hardwareKey"
    ],
    "secretFieldKey": [
      "password",
      "cardNumber",
      "cardVerificationCode",
      "cardPin",
      "passportNumber",
      "licenseNumber",
      "nationalId",
      "apiKeySecret",
      "wifiPassword"
    ],
    "sshCloseReason": [
      "answered",
      "expired",
      "locked"
    ],
    "sshDecision": [
      "allowOnce",
      "allowUntilLock",
      "deny"
    ],
    "strength": [
      "veryWeak",
      "weak",
      "fair",
      "strong",
      "veryStrong"
    ],
    "syncStatus": [
      "notConfigured",
      "offline",
      "syncing",
      "upToDate",
      "error"
    ],
    "textFieldKey": [
      "title",
      "notes",
      "username",
      "cardholderName",
      "cardBrand",
      "cardExpiry",
      "firstName",
      "middleName",
      "lastName",
      "email",
      "phone",
      "company",
      "addressLine1",
      "addressLine2",
      "city",
      "region",
      "postalCode",
      "country",
      "birthDate",
      "apiKeyId",
      "apiKeyHost",
      "wifiSsid",
      "wifiSecurity",
      "sshComment"
    ],
    "theme": [
      "system",
      "light",
      "dark"
    ],
    "trashFilter": [
      "exclude",
      "only",
      "include"
    ],
    "urlMatchMode": [
      "exactOrigin",
      "etld1"
    ]
  },
  "errors": [
    {
      "code": "notImplemented"
    },
    {
      "code": "noAccount"
    },
    {
      "code": "accountExists"
    },
    {
      "code": "locked"
    },
    {
      "code": "alreadyUnlocked"
    },
    {
      "code": "wrongCredentials"
    },
    {
      "code": "secretKeyRequired"
    },
    {
      "code": "secretKeyMalformed"
    },
    {
      "code": "secretKeyMistyped"
    },
    {
      "code": "emptyPassword"
    },
    {
      "code": "quickUnlockUnavailable"
    },
    {
      "code": "storeInUse"
    },
    {
      "code": "storeUnavailable"
    },
    {
      "code": "vaultNotFound"
    },
    {
      "code": "vaultUnavailable"
    },
    {
      "code": "itemNotFound"
    },
    {
      "code": "itemDeleted"
    },
    {
      "code": "sameVault"
    },
    {
      "code": "noOtp"
    },
    {
      "code": "fieldEmpty"
    },
    {
      "code": "invalidInput"
    },
    {
      "code": "reauthRequired"
    },
    {
      "code": "attachmentTooLarge"
    },
    {
      "code": "attachmentQuota"
    },
    {
      "code": "clipboardUnavailable"
    },
    {
      "code": "importFileUnreadable"
    },
    {
      "code": "importFormatInvalid"
    },
    {
      "code": "importPreviewExpired"
    },
    {
      "code": "sshRequestExpired"
    },
    {
      "code": "sshAgentUnavailable"
    },
    {
      "code": "busy"
    },
    {
      "code": "internal"
    }
  ],
  "events": [
    {
      "reason": "idle",
      "type": "locked"
    },
    {
      "state": {
        "error": "internal",
        "lastSyncedAt": 1761480000000,
        "status": "error"
      },
      "type": "syncStateChanged"
    },
    {
      "state": {
        "error": null,
        "lastSyncedAt": null,
        "status": "notConfigured"
      },
      "type": "syncStateChanged"
    },
    {
      "type": "clipboardCleared"
    },
    {
      "request": {
        "client": {
          "pid": 4242,
          "processName": "git"
        },
        "expiresAt": 1761480060000,
        "fingerprint": "SHA256:uNiVztksCsDhcc0u9e8BujQXVUpKZIDTMczCvj3tD2s",
        "itemId": "019a2b3c-4d5e-7020-8000-000000000003",
        "keyTitle": "Deploy key",
        "requestId": "ssh-1",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      },
      "type": "sshApprovalRequested"
    },
    {
      "reason": "expired",
      "requestId": "ssh-1",
      "type": "sshApprovalClosed"
    }
  ],
  "requests": {
    "add_item": [
      {
        "draft": {
          "customFields": [
            {
              "kind": "hidden",
              "label": "PIN",
              "op": "add",
              "value": "1234"
            },
            {
              "elementId": "4-019a2b3c4d5e7000",
              "kind": "email",
              "label": "Recovery email",
              "op": "update",
              "value": "alex@example.org"
            },
            {
              "elementId": "5-019a2b3c4d5e7000",
              "op": "remove"
            }
          ],
          "otp": {
            "op": "set",
            "uri": "otpauth://totp/GitHub:alex?secret=JBSWY3DPEHPK3PXP&issuer=GitHub"
          },
          "secrets": [
            {
              "field": "password",
              "value": "n3w-pa55word"
            }
          ],
          "tags": [
            "work"
          ],
          "text": [
            {
              "field": "title",
              "value": "GitHub"
            },
            {
              "field": "notes",
              "value": null
            }
          ],
          "urls": [
            {
              "allowHttp": false,
              "matchMode": "exactOrigin",
              "op": "add",
              "url": "https://github.com"
            },
            {
              "allowHttp": false,
              "elementId": "3-019a2b3c4d5e7000",
              "matchMode": "etld1",
              "op": "update",
              "url": "https://github.com/login"
            },
            {
              "elementId": "6-019a2b3c4d5e7000",
              "op": "remove"
            }
          ]
        },
        "kind": "login",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "clear_clipboard": [
      null
    ],
    "copy_to_clipboard": [
      {
        "source": {
          "field": {
            "kind": "username"
          },
          "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
          "kind": "item",
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        }
      },
      {
        "source": {
          "field": {
            "kind": "password"
          },
          "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
          "kind": "item",
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        }
      },
      {
        "source": {
          "field": {
            "kind": "totp"
          },
          "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
          "kind": "item",
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        }
      },
      {
        "source": {
          "field": {
            "field": "email",
            "kind": "text"
          },
          "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
          "kind": "item",
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        }
      },
      {
        "source": {
          "field": {
            "field": "cardNumber",
            "kind": "secret"
          },
          "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
          "kind": "item",
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        }
      },
      {
        "source": {
          "field": {
            "elementId": "5-019a2b3c4d5e7000",
            "kind": "customField"
          },
          "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
          "kind": "item",
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        }
      },
      {
        "source": {
          "kind": "generated"
        }
      }
    ],
    "create_account": [
      {
        "defaultVaultName": "Personal",
        "email": "alex@example.com",
        "password": "correct horse battery staple"
      }
    ],
    "create_vault": [
      {
        "name": "Work"
      }
    ],
    "generate_password": [
      {
        "digits": true,
        "excludeAmbiguous": false,
        "kind": "password",
        "length": 20,
        "lowercase": true,
        "symbols": true,
        "uppercase": true
      },
      {
        "appendDigit": false,
        "capitalize": true,
        "kind": "passphrase",
        "separator": "-",
        "words": 5
      },
      {
        "host": "example.com",
        "kind": "forSite"
      }
    ],
    "get_item": [
      {
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "get_settings": [
      null
    ],
    "import_cancel": [
      {
        "previewId": "p-1"
      }
    ],
    "import_choose_file": [
      {
        "format": "enpass"
      },
      {
        "format": "bitwarden"
      },
      {
        "format": "keepass"
      },
      {
        "format": "onePassword"
      },
      {
        "format": "csv"
      }
    ],
    "import_commit": [
      {
        "previewId": "p-1",
        "skip": [
          0
        ],
        "targetVaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "import_preview": [
      {
        "fileToken": "f-1",
        "format": "keepass",
        "password": "kdbx password"
      },
      {
        "fileToken": "f-1",
        "format": "enpass",
        "password": null
      }
    ],
    "list_items": [
      {
        "favoritesOnly": true,
        "query": "git",
        "trash": "exclude",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      },
      {
        "favoritesOnly": false,
        "query": null,
        "trash": "only",
        "vaultId": null
      }
    ],
    "list_vaults": [
      null
    ],
    "lock": [
      null
    ],
    "move_item": [
      {
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "targetVaultId": "019a2b3c-4d5e-7010-8000-000000000002",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "purge_item": [
      {
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "rename_vault": [
      {
        "name": "Family",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "restore_item": [
      {
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "reveal_secret": [
      {
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "secret": {
          "field": "password",
          "kind": "field"
        },
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      },
      {
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "secret": {
          "elementId": "5-019a2b3c4d5e7000",
          "kind": "customField"
        },
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      },
      {
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "secret": {
          "index": 0,
          "kind": "history"
        },
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "security_report": [
      null
    ],
    "session_state": [
      null
    ],
    "set_favorite": [
      {
        "favorite": true,
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "ssh_agent_set_enabled": [
      {
        "enabled": true
      }
    ],
    "ssh_agent_status": [
      null
    ],
    "ssh_respond": [
      {
        "decision": "allowOnce",
        "requestId": "ssh-1"
      },
      {
        "decision": "allowUntilLock",
        "requestId": "ssh-1"
      },
      {
        "decision": "deny",
        "requestId": "ssh-1"
      }
    ],
    "totp_code": [
      {
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "trash_item": [
      {
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "unlock": [
      {
        "password": "correct horse battery staple",
        "secretKey": "M1-7KQ4-X29H-DM3P-RT6W-ZB04-NCFG-8VJ2"
      },
      {
        "password": "correct horse battery staple",
        "secretKey": null
      }
    ],
    "unlock_quick": [
      {
        "method": "biometric"
      },
      {
        "method": "hardwareKey"
      }
    ],
    "update_item": [
      {
        "draft": {
          "customFields": [
            {
              "kind": "hidden",
              "label": "PIN",
              "op": "add",
              "value": "1234"
            },
            {
              "elementId": "4-019a2b3c4d5e7000",
              "kind": "email",
              "label": "Recovery email",
              "op": "update",
              "value": "alex@example.org"
            },
            {
              "elementId": "5-019a2b3c4d5e7000",
              "op": "remove"
            }
          ],
          "otp": {
            "op": "set",
            "uri": "otpauth://totp/GitHub:alex?secret=JBSWY3DPEHPK3PXP"
          },
          "secrets": [
            {
              "field": "password",
              "value": "n3w-pa55word"
            }
          ],
          "tags": [
            "work"
          ],
          "text": [
            {
              "field": "title",
              "value": "GitHub"
            },
            {
              "field": "notes",
              "value": null
            }
          ],
          "urls": [
            {
              "allowHttp": false,
              "matchMode": "exactOrigin",
              "op": "add",
              "url": "https://github.com"
            },
            {
              "allowHttp": false,
              "elementId": "3-019a2b3c4d5e7000",
              "matchMode": "etld1",
              "op": "update",
              "url": "https://github.com/login"
            },
            {
              "elementId": "6-019a2b3c4d5e7000",
              "op": "remove"
            }
          ]
        },
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      },
      {
        "draft": {
          "customFields": [
            {
              "kind": "hidden",
              "label": "PIN",
              "op": "add",
              "value": "1234"
            },
            {
              "elementId": "4-019a2b3c4d5e7000",
              "kind": "email",
              "label": "Recovery email",
              "op": "update",
              "value": "alex@example.org"
            },
            {
              "elementId": "5-019a2b3c4d5e7000",
              "op": "remove"
            }
          ],
          "otp": {
            "op": "remove"
          },
          "secrets": [
            {
              "field": "password",
              "value": "n3w-pa55word"
            }
          ],
          "tags": [
            "work"
          ],
          "text": [
            {
              "field": "title",
              "value": "GitHub"
            },
            {
              "field": "notes",
              "value": null
            }
          ],
          "urls": [
            {
              "allowHttp": false,
              "matchMode": "exactOrigin",
              "op": "add",
              "url": "https://github.com"
            },
            {
              "allowHttp": false,
              "elementId": "3-019a2b3c4d5e7000",
              "matchMode": "etld1",
              "op": "update",
              "url": "https://github.com/login"
            },
            {
              "elementId": "6-019a2b3c4d5e7000",
              "op": "remove"
            }
          ]
        },
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "update_settings": [
      {
        "password": null,
        "settings": {
          "autoLockMinutes": 10,
          "clipboardClearSeconds": 30,
          "language": "system",
          "lockOnScreenLock": true,
          "lockOnSleep": true,
          "theme": "system"
        }
      },
      {
        "password": "correct horse battery staple",
        "settings": {
          "autoLockMinutes": null,
          "clipboardClearSeconds": null,
          "language": "de",
          "lockOnScreenLock": false,
          "lockOnSleep": false,
          "theme": "dark"
        }
      }
    ],
    "verify_emergency_kit": [
      {
        "lastGroup": "8VJ2"
      }
    ]
  },
  "responses": {
    "add_item": [
      {
        "attachments": [
          {
            "attachmentId": "019a2b3c-4d5e-7040-8000-000000000001",
            "fileName": "recovery-codes.txt",
            "mime": "text/plain",
            "size": 512
          }
        ],
        "createdAt": 1761400000000,
        "customFields": [
          {
            "elementId": "4-019a2b3c4d5e7000",
            "kind": "email",
            "label": "Recovery email",
            "value": "alex@example.org"
          },
          {
            "elementId": "5-019a2b3c4d5e7000",
            "kind": "hidden",
            "label": "PIN",
            "value": null
          }
        ],
        "historyCount": 2,
        "otp": {
          "digits": 6,
          "hardwareProtected": false,
          "issuer": "GitHub",
          "period": 30,
          "scheme": "totp"
        },
        "review": {
          "conflictCopyOf": null,
          "conflictingFields": 0,
          "deletedConcurrently": false
        },
        "secrets": [
          "password"
        ],
        "summary": {
          "favorite": true,
          "hasOtp": true,
          "host": "github.com",
          "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
          "kind": "login",
          "modifiedAt": 1761480000000,
          "needsReview": false,
          "subtitle": "alex@example.com",
          "tags": [
            "work"
          ],
          "title": "GitHub",
          "trashedAt": null,
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        },
        "text": [
          {
            "field": "username",
            "value": "alex@example.com"
          }
        ],
        "urls": [
          {
            "allowHttp": false,
            "elementId": "3-019a2b3c4d5e7000",
            "matchMode": "exactOrigin",
            "url": "https://github.com/login"
          }
        ]
      }
    ],
    "clear_clipboard": [
      null
    ],
    "copy_to_clipboard": [
      {
        "clearAfterSeconds": 30,
        "clearsAt": 1761480030000
      },
      {
        "clearAfterSeconds": null,
        "clearsAt": null
      }
    ],
    "create_account": [
      {
        "emergencyKit": {
          "accountId": "019a2b3c-4d5e-7030-8000-000000000001",
          "email": "alex@example.com",
          "secretKey": "M1-7KQ4-X29H-DM3P-RT6W-ZB04-NCFG-8VJ2"
        },
        "session": {
          "accountId": "019a2b3c-4d5e-7030-8000-000000000001",
          "email": "alex@example.com",
          "state": "unlocked"
        }
      }
    ],
    "create_vault": [
      {
        "available": true,
        "itemCount": 42,
        "name": "Personal",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      },
      {
        "available": false,
        "itemCount": 0,
        "name": null,
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000002"
      }
    ],
    "generate_password": [
      {
        "entropyBits": 131.1,
        "strength": "veryStrong",
        "value": "q7#Vt2!mZp9@Lx4$Nw6&"
      }
    ],
    "get_item": [
      {
        "attachments": [
          {
            "attachmentId": "019a2b3c-4d5e-7040-8000-000000000001",
            "fileName": "recovery-codes.txt",
            "mime": "text/plain",
            "size": 512
          }
        ],
        "createdAt": 1761400000000,
        "customFields": [
          {
            "elementId": "4-019a2b3c4d5e7000",
            "kind": "email",
            "label": "Recovery email",
            "value": "alex@example.org"
          },
          {
            "elementId": "5-019a2b3c4d5e7000",
            "kind": "hidden",
            "label": "PIN",
            "value": null
          }
        ],
        "historyCount": 2,
        "otp": {
          "digits": 6,
          "hardwareProtected": false,
          "issuer": "GitHub",
          "period": 30,
          "scheme": "totp"
        },
        "review": {
          "conflictCopyOf": null,
          "conflictingFields": 0,
          "deletedConcurrently": false
        },
        "secrets": [
          "password"
        ],
        "summary": {
          "favorite": true,
          "hasOtp": true,
          "host": "github.com",
          "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
          "kind": "login",
          "modifiedAt": 1761480000000,
          "needsReview": false,
          "subtitle": "alex@example.com",
          "tags": [
            "work"
          ],
          "title": "GitHub",
          "trashedAt": null,
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        },
        "text": [
          {
            "field": "username",
            "value": "alex@example.com"
          }
        ],
        "urls": [
          {
            "allowHttp": false,
            "elementId": "3-019a2b3c4d5e7000",
            "matchMode": "exactOrigin",
            "url": "https://github.com/login"
          }
        ]
      }
    ],
    "get_settings": [
      {
        "autoLockMinutes": 10,
        "clipboardClearSeconds": 30,
        "language": "system",
        "lockOnScreenLock": true,
        "lockOnSleep": true,
        "theme": "system"
      },
      {
        "autoLockMinutes": null,
        "clipboardClearSeconds": null,
        "language": "de",
        "lockOnScreenLock": false,
        "lockOnSleep": false,
        "theme": "dark"
      }
    ],
    "import_cancel": [
      null
    ],
    "import_choose_file": [
      {
        "fileName": "enpass-export.json",
        "fileToken": "f-1",
        "kind": "chosen"
      },
      {
        "kind": "cancelled"
      }
    ],
    "import_commit": [
      {
        "imported": 1,
        "skipped": 1
      }
    ],
    "import_preview": [
      {
        "entries": [
          {
            "duplicateOf": {
              "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
              "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
            },
            "index": 0,
            "kind": "login",
            "subtitle": "alex@example.com",
            "title": "GitHub"
          },
          {
            "duplicateOf": null,
            "index": 1,
            "kind": "secureNote",
            "subtitle": null,
            "title": null
          }
        ],
        "previewId": "p-1",
        "warnings": [
          "unmappedFields",
          "attachmentsSkipped",
          "entriesSkipped"
        ]
      }
    ],
    "list_items": [
      [
        {
          "favorite": true,
          "hasOtp": true,
          "host": "github.com",
          "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
          "kind": "login",
          "modifiedAt": 1761480000000,
          "needsReview": false,
          "subtitle": "alex@example.com",
          "tags": [
            "work"
          ],
          "title": "GitHub",
          "trashedAt": null,
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        },
        {
          "favorite": false,
          "hasOtp": false,
          "host": null,
          "itemId": "019a2b3c-4d5e-7020-8000-000000000002",
          "kind": "secureNote",
          "modifiedAt": null,
          "needsReview": true,
          "subtitle": null,
          "tags": [],
          "title": null,
          "trashedAt": 1761480000000,
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        }
      ]
    ],
    "list_vaults": [
      [
        {
          "available": true,
          "itemCount": 42,
          "name": "Personal",
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        },
        {
          "available": false,
          "itemCount": 0,
          "name": null,
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000002"
        }
      ]
    ],
    "lock": [
      {
        "state": "noAccount"
      },
      {
        "email": "alex@example.com",
        "quickUnlock": [
          "biometric",
          "hardwareKey"
        ],
        "secretKeyStored": true,
        "state": "locked"
      },
      {
        "email": null,
        "quickUnlock": [],
        "secretKeyStored": false,
        "state": "locked"
      },
      {
        "accountId": "019a2b3c-4d5e-7030-8000-000000000001",
        "email": "alex@example.com",
        "state": "unlocked"
      }
    ],
    "move_item": [
      {
        "itemId": "019a2b3c-4d5e-7020-8000-000000000009",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000002"
      }
    ],
    "purge_item": [
      null
    ],
    "rename_vault": [
      {
        "available": true,
        "itemCount": 42,
        "name": "Personal",
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      },
      {
        "available": false,
        "itemCount": 0,
        "name": null,
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000002"
      }
    ],
    "restore_item": [
      null
    ],
    "reveal_secret": [
      {
        "hideAfterSeconds": 60,
        "value": "hunter2"
      }
    ],
    "security_report": [
      {
        "counts": {
          "high": 2,
          "insecureUrl": 1,
          "low": 0,
          "medium": 0,
          "missingTotp": 2,
          "old": 1,
          "reused": 2,
          "reusedGroups": 1,
          "weak": 1
        },
        "findings": [
          {
            "insecureHost": "shop.example.com",
            "issues": [
              {
                "action": "changePassword",
                "kind": "weak",
                "severity": "high"
              },
              {
                "action": "changePassword",
                "kind": "reused",
                "severity": "high"
              },
              {
                "action": "setUpTotp",
                "kind": "missingTotp",
                "severity": "medium"
              },
              {
                "action": "reviewUrl",
                "kind": "insecureUrl",
                "severity": "medium"
              },
              {
                "action": "changePassword",
                "kind": "old",
                "severity": "low"
              }
            ],
            "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
            "reusedWith": [
              {
                "itemId": "019a2b3c-4d5e-7020-8000-000000000002",
                "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
              }
            ],
            "totpSite": {
              "documentation": "https://docs.github.com/en/authentication/securing-your-account-with-two-factor-authentication-2fa",
              "name": "GitHub"
            },
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
          },
          {
            "insecureHost": null,
            "issues": [
              {
                "action": "changePassword",
                "kind": "reused",
                "severity": "high"
              },
              {
                "action": "setUpTotp",
                "kind": "missingTotp",
                "severity": "low"
              }
            ],
            "itemId": "019a2b3c-4d5e-7020-8000-000000000002",
            "reusedWith": [
              {
                "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
                "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
              }
            ],
            "totpSite": {
              "documentation": null,
              "name": "Example"
            },
            "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
          }
        ],
        "generatedAt": 1761480000000,
        "itemsChecked": 42
      }
    ],
    "session_state": [
      {
        "state": "noAccount"
      },
      {
        "email": "alex@example.com",
        "quickUnlock": [
          "biometric",
          "hardwareKey"
        ],
        "secretKeyStored": true,
        "state": "locked"
      },
      {
        "email": null,
        "quickUnlock": [],
        "secretKeyStored": false,
        "state": "locked"
      },
      {
        "accountId": "019a2b3c-4d5e-7030-8000-000000000001",
        "email": "alex@example.com",
        "state": "unlocked"
      }
    ],
    "set_favorite": [
      {
        "favorite": true,
        "hasOtp": true,
        "host": "github.com",
        "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
        "kind": "login",
        "modifiedAt": 1761480000000,
        "needsReview": false,
        "subtitle": "alex@example.com",
        "tags": [
          "work"
        ],
        "title": "GitHub",
        "trashedAt": null,
        "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
      }
    ],
    "ssh_agent_set_enabled": [
      {
        "enabled": true,
        "endpoint": "\\\\.\\pipe\\openssh-ssh-agent",
        "keyCount": 2,
        "running": true
      },
      {
        "enabled": false,
        "endpoint": null,
        "keyCount": 0,
        "running": false
      }
    ],
    "ssh_agent_status": [
      {
        "enabled": true,
        "endpoint": "\\\\.\\pipe\\openssh-ssh-agent",
        "keyCount": 2,
        "running": true
      },
      {
        "enabled": false,
        "endpoint": null,
        "keyCount": 0,
        "running": false
      }
    ],
    "ssh_respond": [
      null
    ],
    "totp_code": [
      {
        "code": "492039",
        "period": 30,
        "secondsRemaining": 17
      },
      {
        "code": "755224",
        "period": null,
        "secondsRemaining": null
      }
    ],
    "trash_item": [
      null
    ],
    "unlock": [
      {
        "state": "noAccount"
      },
      {
        "email": "alex@example.com",
        "quickUnlock": [
          "biometric",
          "hardwareKey"
        ],
        "secretKeyStored": true,
        "state": "locked"
      },
      {
        "email": null,
        "quickUnlock": [],
        "secretKeyStored": false,
        "state": "locked"
      },
      {
        "accountId": "019a2b3c-4d5e-7030-8000-000000000001",
        "email": "alex@example.com",
        "state": "unlocked"
      }
    ],
    "unlock_quick": [
      {
        "state": "noAccount"
      },
      {
        "email": "alex@example.com",
        "quickUnlock": [
          "biometric",
          "hardwareKey"
        ],
        "secretKeyStored": true,
        "state": "locked"
      },
      {
        "email": null,
        "quickUnlock": [],
        "secretKeyStored": false,
        "state": "locked"
      },
      {
        "accountId": "019a2b3c-4d5e-7030-8000-000000000001",
        "email": "alex@example.com",
        "state": "unlocked"
      }
    ],
    "update_item": [
      {
        "attachments": [
          {
            "attachmentId": "019a2b3c-4d5e-7040-8000-000000000001",
            "fileName": "recovery-codes.txt",
            "mime": "text/plain",
            "size": 512
          }
        ],
        "createdAt": 1761400000000,
        "customFields": [
          {
            "elementId": "4-019a2b3c4d5e7000",
            "kind": "email",
            "label": "Recovery email",
            "value": "alex@example.org"
          },
          {
            "elementId": "5-019a2b3c4d5e7000",
            "kind": "hidden",
            "label": "PIN",
            "value": null
          }
        ],
        "historyCount": 2,
        "otp": {
          "digits": 6,
          "hardwareProtected": false,
          "issuer": "GitHub",
          "period": 30,
          "scheme": "totp"
        },
        "review": {
          "conflictCopyOf": null,
          "conflictingFields": 0,
          "deletedConcurrently": false
        },
        "secrets": [
          "password"
        ],
        "summary": {
          "favorite": true,
          "hasOtp": true,
          "host": "github.com",
          "itemId": "019a2b3c-4d5e-7020-8000-000000000001",
          "kind": "login",
          "modifiedAt": 1761480000000,
          "needsReview": false,
          "subtitle": "alex@example.com",
          "tags": [
            "work"
          ],
          "title": "GitHub",
          "trashedAt": null,
          "vaultId": "019a2b3c-4d5e-7010-8000-000000000001"
        },
        "text": [
          {
            "field": "username",
            "value": "alex@example.com"
          }
        ],
        "urls": [
          {
            "allowHttp": false,
            "elementId": "3-019a2b3c4d5e7000",
            "matchMode": "exactOrigin",
            "url": "https://github.com/login"
          }
        ]
      }
    ],
    "update_settings": [
      {
        "autoLockMinutes": 10,
        "clipboardClearSeconds": 30,
        "language": "system",
        "lockOnScreenLock": true,
        "lockOnSleep": true,
        "theme": "system"
      },
      {
        "autoLockMinutes": null,
        "clipboardClearSeconds": null,
        "language": "de",
        "lockOnScreenLock": false,
        "lockOnSleep": false,
        "theme": "dark"
      }
    ],
    "verify_emergency_kit": [
      {
        "matches": true
      },
      {
        "matches": false
      }
    ]
  }
} satisfies ContractFixtures;
