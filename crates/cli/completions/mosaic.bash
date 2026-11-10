_mosaic() {
  local cur="${COMP_WORDS[COMP_CWORD]}" cmd="${COMP_WORDS[1]}"
  if [ "$COMP_CWORD" -le 1 ]; then
    COMPREPLY=($(compgen -W "unlock lock status list search get totp generate run inject ssh import export sync setup recover completions help" -- "$cur"))
    return
  fi
  case "$cmd" in
    unlock) COMPREPLY=($(compgen -W "--ttl --password-stdin --password-fd --secret-key-prompt --store --profile --json --lang --no-input --quiet" -- "$cur")) ;;
    lock) COMPREPLY=($(compgen -W "--store --profile --json --lang --no-input --quiet" -- "$cur")) ;;
    status) COMPREPLY=($(compgen -W "--store --profile --json --lang --no-input --quiet" -- "$cur")) ;;
    list) COMPREPLY=($(compgen -W "--vault --type --favorites --trash --store --profile --json --lang --no-input --quiet" -- "$cur")) ;;
    search) COMPREPLY=($(compgen -W "--vault --store --profile --json --lang --no-input --quiet" -- "$cur")) ;;
    get) COMPREPLY=($(compgen -W "--field --reveal --copy --clear-after --store --profile --json --lang --no-input --quiet" -- "$cur")) ;;
    totp) COMPREPLY=($(compgen -W "--copy --clear-after --store --profile --json --lang --no-input --quiet" -- "$cur")) ;;
    generate) COMPREPLY=($(compgen -W "--length --words --separator --no-upper --no-lower --no-digits --no-symbols --site --copy --clear-after --store --profile --json --lang --no-input --quiet" -- "$cur")) ;;
    run) COMPREPLY=($(compgen -W "--env --env-file --store --profile --json --lang --no-input --quiet" -- "$cur")) ;;
    inject) COMPREPLY=($(compgen -W "--in --out --reveal --store --profile --json --lang --no-input --quiet" -- "$cur")) ;;
    ssh) COMPREPLY=($(compgen -W "--store --profile --json --lang --no-input --quiet list public-key agent-status" -- "$cur")) ;;
    import) COMPREPLY=($(compgen -W "--vault --dry-run --file-password-prompt --store --profile --json --lang --no-input --quiet enpass bitwarden keepass 1password cxf csv mosaicx" -- "$cur")) ;;
    export) COMPREPLY=($(compgen -W "--vault --include-attachments --store --profile --json --lang --no-input --quiet" -- "$cur")) ;;
    sync) COMPREPLY=($(compgen -W "--status --acknowledge --store --profile --json --lang --no-input --quiet" -- "$cur")) ;;
    setup) COMPREPLY=($(compgen -W "--folder --server --email --store --profile --json --lang --no-input --quiet own-store" -- "$cur")) ;;
    recover) COMPREPLY=($(compgen -W "--store --profile --json --lang --no-input --quiet" -- "$cur")) ;;
    completions) COMPREPLY=($(compgen -W "--store --profile --json --lang --no-input --quiet bash zsh fish powershell" -- "$cur")) ;;
    help) COMPREPLY=($(compgen -W "--man --store --profile --json --lang --no-input --quiet" -- "$cur")) ;;
  esac
}
complete -F _mosaic mosaic
