
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'dot' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'dot'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value.StartsWith('-') -or
                $element.Value -eq $wordToComplete) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'dot' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Add files to be tracked')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Show the working tree status')
            [CompletionResult]::new('commit', 'commit', [CompletionResultType]::ParameterValue, 'Record changes to the repository')
            [CompletionResult]::new('checkout', 'checkout', [CompletionResultType]::ParameterValue, 'Switch branches or restore working tree files')
            [CompletionResult]::new('reset', 'reset', [CompletionResultType]::ParameterValue, 'Reset current HEAD to the specified state')
            [CompletionResult]::new('push', 'push', [CompletionResultType]::ParameterValue, 'Update remote refs along with associated objects')
            [CompletionResult]::new('pull', 'pull', [CompletionResultType]::ParameterValue, 'Fetch from and integrate with another repository')
            [CompletionResult]::new('init', 'init', [CompletionResultType]::ParameterValue, 'Initialize a new dotman repository')
            [CompletionResult]::new('show', 'show', [CompletionResultType]::ParameterValue, 'Show various types of objects')
            [CompletionResult]::new('log', 'log', [CompletionResultType]::ParameterValue, 'Show commit logs')
            [CompletionResult]::new('diff', 'diff', [CompletionResultType]::ParameterValue, 'Show changes between commits')
            [CompletionResult]::new('rm', 'rm', [CompletionResultType]::ParameterValue, 'Remove files from tracking')
            [CompletionResult]::new('remote', 'remote', [CompletionResultType]::ParameterValue, 'Manage remote repositories')
            [CompletionResult]::new('branch', 'branch', [CompletionResultType]::ParameterValue, 'Manage branches')
            [CompletionResult]::new('config', 'config', [CompletionResultType]::ParameterValue, 'Get and set repository or user options')
            [CompletionResult]::new('completion', 'completion', [CompletionResultType]::ParameterValue, 'Generate shell completion scripts')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'dot;add' {
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'f')
            [CompletionResult]::new('--force', '--force', [CompletionResultType]::ParameterName, 'force')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'dot;status' {
            [CompletionResult]::new('-s', '-s', [CompletionResultType]::ParameterName, 's')
            [CompletionResult]::new('--short', '--short', [CompletionResultType]::ParameterName, 'short')
            [CompletionResult]::new('-u', '-u', [CompletionResultType]::ParameterName, 'u')
            [CompletionResult]::new('--untracked', '--untracked', [CompletionResultType]::ParameterName, 'untracked')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'dot;commit' {
            [CompletionResult]::new('-m', '-m', [CompletionResultType]::ParameterName, 'm')
            [CompletionResult]::new('--message', '--message', [CompletionResultType]::ParameterName, 'message')
            [CompletionResult]::new('-a', '-a', [CompletionResultType]::ParameterName, 'a')
            [CompletionResult]::new('--all', '--all', [CompletionResultType]::ParameterName, 'all')
            [CompletionResult]::new('--amend', '--amend', [CompletionResultType]::ParameterName, 'Amend the previous commit')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'dot;checkout' {
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'f')
            [CompletionResult]::new('--force', '--force', [CompletionResultType]::ParameterName, 'force')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'dot;reset' {
            [CompletionResult]::new('--hard', '--hard', [CompletionResultType]::ParameterName, 'hard')
            [CompletionResult]::new('--soft', '--soft', [CompletionResultType]::ParameterName, 'soft')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'dot;push' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'dot;pull' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'dot;init' {
            [CompletionResult]::new('-b', '-b', [CompletionResultType]::ParameterName, 'b')
            [CompletionResult]::new('--bare', '--bare', [CompletionResultType]::ParameterName, 'bare')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'dot;show' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'dot;log' {
            [CompletionResult]::new('-n', '-n', [CompletionResultType]::ParameterName, 'n')
            [CompletionResult]::new('--limit', '--limit', [CompletionResultType]::ParameterName, 'limit')
            [CompletionResult]::new('--oneline', '--oneline', [CompletionResultType]::ParameterName, 'oneline')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'dot;diff' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'dot;rm' {
            [CompletionResult]::new('-c', '-c', [CompletionResultType]::ParameterName, 'c')
            [CompletionResult]::new('--cached', '--cached', [CompletionResultType]::ParameterName, 'cached')
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'f')
            [CompletionResult]::new('--force', '--force', [CompletionResultType]::ParameterName, 'force')
            [CompletionResult]::new('-i', '-i', [CompletionResultType]::ParameterName, 'i')
            [CompletionResult]::new('--interactive', '--interactive', [CompletionResultType]::ParameterName, 'interactive')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'dot;remote' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all remotes')
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Add a new remote')
            [CompletionResult]::new('remove', 'remove', [CompletionResultType]::ParameterValue, 'Remove a remote')
            [CompletionResult]::new('set-url', 'set-url', [CompletionResultType]::ParameterValue, 'Set the URL for a remote')
            [CompletionResult]::new('show', 'show', [CompletionResultType]::ParameterValue, 'Show information about a remote')
            [CompletionResult]::new('rename', 'rename', [CompletionResultType]::ParameterValue, 'Rename a remote')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'dot;remote;list' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'dot;remote;add' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'dot;remote;remove' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'dot;remote;set-url' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'dot;remote;show' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'dot;remote;rename' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'dot;remote;help' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all remotes')
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Add a new remote')
            [CompletionResult]::new('remove', 'remove', [CompletionResultType]::ParameterValue, 'Remove a remote')
            [CompletionResult]::new('set-url', 'set-url', [CompletionResultType]::ParameterValue, 'Set the URL for a remote')
            [CompletionResult]::new('show', 'show', [CompletionResultType]::ParameterValue, 'Show information about a remote')
            [CompletionResult]::new('rename', 'rename', [CompletionResultType]::ParameterValue, 'Rename a remote')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'dot;remote;help;list' {
            break
        }
        'dot;remote;help;add' {
            break
        }
        'dot;remote;help;remove' {
            break
        }
        'dot;remote;help;set-url' {
            break
        }
        'dot;remote;help;show' {
            break
        }
        'dot;remote;help;rename' {
            break
        }
        'dot;remote;help;help' {
            break
        }
        'dot;branch' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all branches')
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create a new branch')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Delete a branch')
            [CompletionResult]::new('rename', 'rename', [CompletionResultType]::ParameterValue, 'Rename a branch')
            [CompletionResult]::new('set-upstream', 'set-upstream', [CompletionResultType]::ParameterValue, 'Set upstream tracking for a branch')
            [CompletionResult]::new('unset-upstream', 'unset-upstream', [CompletionResultType]::ParameterValue, 'Remove upstream tracking for a branch')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'dot;branch;list' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'dot;branch;create' {
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'Starting point (commit or branch)')
            [CompletionResult]::new('--from', '--from', [CompletionResultType]::ParameterName, 'Starting point (commit or branch)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'dot;branch;delete' {
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'Force deletion')
            [CompletionResult]::new('--force', '--force', [CompletionResultType]::ParameterName, 'Force deletion')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'dot;branch;rename' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'dot;branch;set-upstream' {
            [CompletionResult]::new('-b', '-b', [CompletionResultType]::ParameterName, 'Branch name (current branch if not specified)')
            [CompletionResult]::new('--branch', '--branch', [CompletionResultType]::ParameterName, 'Branch name (current branch if not specified)')
            [CompletionResult]::new('-b', '-b', [CompletionResultType]::ParameterName, 'Remote branch name (same as local branch if not specified)')
            [CompletionResult]::new('--remote-branch', '--remote-branch', [CompletionResultType]::ParameterName, 'Remote branch name (same as local branch if not specified)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'dot;branch;unset-upstream' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'dot;branch;help' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all branches')
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create a new branch')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Delete a branch')
            [CompletionResult]::new('rename', 'rename', [CompletionResultType]::ParameterValue, 'Rename a branch')
            [CompletionResult]::new('set-upstream', 'set-upstream', [CompletionResultType]::ParameterValue, 'Set upstream tracking for a branch')
            [CompletionResult]::new('unset-upstream', 'unset-upstream', [CompletionResultType]::ParameterValue, 'Remove upstream tracking for a branch')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'dot;branch;help;list' {
            break
        }
        'dot;branch;help;create' {
            break
        }
        'dot;branch;help;delete' {
            break
        }
        'dot;branch;help;rename' {
            break
        }
        'dot;branch;help;set-upstream' {
            break
        }
        'dot;branch;help;unset-upstream' {
            break
        }
        'dot;branch;help;help' {
            break
        }
        'dot;config' {
            [CompletionResult]::new('--unset', '--unset', [CompletionResultType]::ParameterName, 'Unset the configuration key')
            [CompletionResult]::new('-l', '-l', [CompletionResultType]::ParameterName, 'List all configuration values')
            [CompletionResult]::new('--list', '--list', [CompletionResultType]::ParameterName, 'List all configuration values')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'dot;completion' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'v')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'dot;help' {
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Add files to be tracked')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Show the working tree status')
            [CompletionResult]::new('commit', 'commit', [CompletionResultType]::ParameterValue, 'Record changes to the repository')
            [CompletionResult]::new('checkout', 'checkout', [CompletionResultType]::ParameterValue, 'Switch branches or restore working tree files')
            [CompletionResult]::new('reset', 'reset', [CompletionResultType]::ParameterValue, 'Reset current HEAD to the specified state')
            [CompletionResult]::new('push', 'push', [CompletionResultType]::ParameterValue, 'Update remote refs along with associated objects')
            [CompletionResult]::new('pull', 'pull', [CompletionResultType]::ParameterValue, 'Fetch from and integrate with another repository')
            [CompletionResult]::new('init', 'init', [CompletionResultType]::ParameterValue, 'Initialize a new dotman repository')
            [CompletionResult]::new('show', 'show', [CompletionResultType]::ParameterValue, 'Show various types of objects')
            [CompletionResult]::new('log', 'log', [CompletionResultType]::ParameterValue, 'Show commit logs')
            [CompletionResult]::new('diff', 'diff', [CompletionResultType]::ParameterValue, 'Show changes between commits')
            [CompletionResult]::new('rm', 'rm', [CompletionResultType]::ParameterValue, 'Remove files from tracking')
            [CompletionResult]::new('remote', 'remote', [CompletionResultType]::ParameterValue, 'Manage remote repositories')
            [CompletionResult]::new('branch', 'branch', [CompletionResultType]::ParameterValue, 'Manage branches')
            [CompletionResult]::new('config', 'config', [CompletionResultType]::ParameterValue, 'Get and set repository or user options')
            [CompletionResult]::new('completion', 'completion', [CompletionResultType]::ParameterValue, 'Generate shell completion scripts')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'dot;help;add' {
            break
        }
        'dot;help;status' {
            break
        }
        'dot;help;commit' {
            break
        }
        'dot;help;checkout' {
            break
        }
        'dot;help;reset' {
            break
        }
        'dot;help;push' {
            break
        }
        'dot;help;pull' {
            break
        }
        'dot;help;init' {
            break
        }
        'dot;help;show' {
            break
        }
        'dot;help;log' {
            break
        }
        'dot;help;diff' {
            break
        }
        'dot;help;rm' {
            break
        }
        'dot;help;remote' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all remotes')
            [CompletionResult]::new('add', 'add', [CompletionResultType]::ParameterValue, 'Add a new remote')
            [CompletionResult]::new('remove', 'remove', [CompletionResultType]::ParameterValue, 'Remove a remote')
            [CompletionResult]::new('set-url', 'set-url', [CompletionResultType]::ParameterValue, 'Set the URL for a remote')
            [CompletionResult]::new('show', 'show', [CompletionResultType]::ParameterValue, 'Show information about a remote')
            [CompletionResult]::new('rename', 'rename', [CompletionResultType]::ParameterValue, 'Rename a remote')
            break
        }
        'dot;help;remote;list' {
            break
        }
        'dot;help;remote;add' {
            break
        }
        'dot;help;remote;remove' {
            break
        }
        'dot;help;remote;set-url' {
            break
        }
        'dot;help;remote;show' {
            break
        }
        'dot;help;remote;rename' {
            break
        }
        'dot;help;branch' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List all branches')
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create a new branch')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Delete a branch')
            [CompletionResult]::new('rename', 'rename', [CompletionResultType]::ParameterValue, 'Rename a branch')
            [CompletionResult]::new('set-upstream', 'set-upstream', [CompletionResultType]::ParameterValue, 'Set upstream tracking for a branch')
            [CompletionResult]::new('unset-upstream', 'unset-upstream', [CompletionResultType]::ParameterValue, 'Remove upstream tracking for a branch')
            break
        }
        'dot;help;branch;list' {
            break
        }
        'dot;help;branch;create' {
            break
        }
        'dot;help;branch;delete' {
            break
        }
        'dot;help;branch;rename' {
            break
        }
        'dot;help;branch;set-upstream' {
            break
        }
        'dot;help;branch;unset-upstream' {
            break
        }
        'dot;help;config' {
            break
        }
        'dot;help;completion' {
            break
        }
        'dot;help;help' {
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
