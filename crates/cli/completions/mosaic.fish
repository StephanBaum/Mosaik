complete -c mosaic -f
complete -c mosaic -n __fish_use_subcommand -a unlock -d 'Unlock with the master password and keep a short session.'
complete -c mosaic -n '__fish_seen_subcommand_from unlock' -l ttl
complete -c mosaic -n '__fish_seen_subcommand_from unlock' -l password-stdin
complete -c mosaic -n '__fish_seen_subcommand_from unlock' -l password-fd
complete -c mosaic -n '__fish_seen_subcommand_from unlock' -l secret-key-prompt
complete -c mosaic -n __fish_use_subcommand -a lock -d 'End the session.'
complete -c mosaic -n __fish_use_subcommand -a status -d 'Show the store, the session and the sync state.'
complete -c mosaic -n __fish_use_subcommand -a list -d 'List items.'
complete -c mosaic -n '__fish_seen_subcommand_from list' -l vault
complete -c mosaic -n '__fish_seen_subcommand_from list' -l type
complete -c mosaic -n '__fish_seen_subcommand_from list' -l favorites
complete -c mosaic -n '__fish_seen_subcommand_from list' -l trash
complete -c mosaic -n __fish_use_subcommand -a search -d 'Search items.'
complete -c mosaic -n '__fish_seen_subcommand_from search' -l vault
complete -c mosaic -n __fish_use_subcommand -a get -d 'Show an item or one field of it.'
complete -c mosaic -n '__fish_seen_subcommand_from get' -l field
complete -c mosaic -n '__fish_seen_subcommand_from get' -l reveal
complete -c mosaic -n '__fish_seen_subcommand_from get' -l copy
complete -c mosaic -n '__fish_seen_subcommand_from get' -l clear-after
complete -c mosaic -n __fish_use_subcommand -a totp -d 'Print the current one-time code.'
complete -c mosaic -n '__fish_seen_subcommand_from totp' -l copy
complete -c mosaic -n '__fish_seen_subcommand_from totp' -l clear-after
complete -c mosaic -n __fish_use_subcommand -a generate -d 'Generate a password or passphrase.'
complete -c mosaic -n '__fish_seen_subcommand_from generate' -l length
complete -c mosaic -n '__fish_seen_subcommand_from generate' -l words
complete -c mosaic -n '__fish_seen_subcommand_from generate' -l separator
complete -c mosaic -n '__fish_seen_subcommand_from generate' -l no-upper
complete -c mosaic -n '__fish_seen_subcommand_from generate' -l no-lower
complete -c mosaic -n '__fish_seen_subcommand_from generate' -l no-digits
complete -c mosaic -n '__fish_seen_subcommand_from generate' -l no-symbols
complete -c mosaic -n '__fish_seen_subcommand_from generate' -l site
complete -c mosaic -n '__fish_seen_subcommand_from generate' -l copy
complete -c mosaic -n '__fish_seen_subcommand_from generate' -l clear-after
complete -c mosaic -n __fish_use_subcommand -a run -d 'Run a command with secrets in its environment.'
complete -c mosaic -n '__fish_seen_subcommand_from run' -l env
complete -c mosaic -n '__fish_seen_subcommand_from run' -l env-file
complete -c mosaic -n __fish_use_subcommand -a inject -d 'Fill {{ mosaic://... }} placeholders of a template.'
complete -c mosaic -n '__fish_seen_subcommand_from inject' -l in
complete -c mosaic -n '__fish_seen_subcommand_from inject' -l out
complete -c mosaic -n '__fish_seen_subcommand_from inject' -l reveal
complete -c mosaic -n __fish_use_subcommand -a ssh -d 'List SSH keys, print a public key, or explain the agent.'
complete -c mosaic -n __fish_use_subcommand -a import -d 'Import a file from another password manager or a Mosaic export.'
complete -c mosaic -n '__fish_seen_subcommand_from import' -l vault
complete -c mosaic -n '__fish_seen_subcommand_from import' -l dry-run
complete -c mosaic -n '__fish_seen_subcommand_from import' -l file-password-prompt
complete -c mosaic -n __fish_use_subcommand -a export -d 'Write an encrypted export file.'
complete -c mosaic -n '__fish_seen_subcommand_from export' -l vault
complete -c mosaic -n '__fish_seen_subcommand_from export' -l include-attachments
complete -c mosaic -n __fish_use_subcommand -a sync -d 'Sync once with the stores backend.'
complete -c mosaic -n '__fish_seen_subcommand_from sync' -l status
complete -c mosaic -n '__fish_seen_subcommand_from sync' -l acknowledge
complete -c mosaic -n __fish_use_subcommand -a setup -d 'Give the CLI its own device store.'
complete -c mosaic -n '__fish_seen_subcommand_from setup' -l folder
complete -c mosaic -n '__fish_seen_subcommand_from setup' -l server
complete -c mosaic -n '__fish_seen_subcommand_from setup' -l email
complete -c mosaic -n __fish_use_subcommand -a recover -d 'Recover after a password or key change made elsewhere.'
complete -c mosaic -n __fish_use_subcommand -a completions -d 'Print a shell completion script.'
complete -c mosaic -n __fish_use_subcommand -a help -d 'Show help or the manual page.'
complete -c mosaic -n '__fish_seen_subcommand_from help' -l man
complete -c mosaic -l store
complete -c mosaic -l profile
complete -c mosaic -l json
complete -c mosaic -l lang
complete -c mosaic -l no-input
complete -c mosaic -l quiet
