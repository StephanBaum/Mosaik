Register-ArgumentCompleter -Native -CommandName mosaic -ScriptBlock {
  param($wordToComplete, $commandAst, $cursorPosition)
  $words = @('unlock','lock','status','list','search','get','totp','generate','run','inject','ssh','import','export','sync','setup','recover','completions','help')
  $words += @('--store','--profile','--json','--lang','--no-input','--quiet')
  $words | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
    [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
  }
}
