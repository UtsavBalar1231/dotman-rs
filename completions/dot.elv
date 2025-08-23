
use builtin;
use str;

set edit:completion:arg-completer[dot] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'dot'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'dot'= {
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
            cand -V 'Print version'
            cand --version 'Print version'
            cand add 'Add files to be tracked'
            cand status 'Show the working tree status'
            cand commit 'Record changes to the repository'
            cand checkout 'Switch branches or restore working tree files'
            cand reset 'Reset current HEAD to the specified state'
            cand push 'Update remote refs along with associated objects'
            cand pull 'Fetch from and integrate with another repository'
            cand init 'Initialize a new dotman repository'
            cand show 'Show various types of objects'
            cand log 'Show commit logs'
            cand diff 'Show changes between commits'
            cand rm 'Remove files from tracking'
            cand remote 'Manage remote repositories'
            cand branch 'Manage branches'
            cand config 'Get and set repository or user options'
            cand completion 'Generate shell completion scripts'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'dot;add'= {
            cand -f 'f'
            cand --force 'force'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;status'= {
            cand -s 's'
            cand --short 'short'
            cand -u 'u'
            cand --untracked 'untracked'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;commit'= {
            cand -m 'm'
            cand --message 'message'
            cand -a 'a'
            cand --all 'all'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;checkout'= {
            cand -f 'f'
            cand --force 'force'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;reset'= {
            cand --hard 'hard'
            cand --soft 'soft'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;push'= {
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;pull'= {
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;init'= {
            cand -b 'b'
            cand --bare 'bare'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;show'= {
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;log'= {
            cand -n 'n'
            cand --limit 'limit'
            cand --oneline 'oneline'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;diff'= {
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;rm'= {
            cand -c 'c'
            cand --cached 'cached'
            cand -f 'f'
            cand --force 'force'
            cand -i 'i'
            cand --interactive 'interactive'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;remote'= {
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
            cand list 'List all remotes'
            cand add 'Add a new remote'
            cand remove 'Remove a remote'
            cand set-url 'Set the URL for a remote'
            cand show 'Show information about a remote'
            cand rename 'Rename a remote'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'dot;remote;list'= {
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;remote;add'= {
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;remote;remove'= {
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;remote;set-url'= {
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;remote;show'= {
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;remote;rename'= {
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;remote;help'= {
            cand list 'List all remotes'
            cand add 'Add a new remote'
            cand remove 'Remove a remote'
            cand set-url 'Set the URL for a remote'
            cand show 'Show information about a remote'
            cand rename 'Rename a remote'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'dot;remote;help;list'= {
        }
        &'dot;remote;help;add'= {
        }
        &'dot;remote;help;remove'= {
        }
        &'dot;remote;help;set-url'= {
        }
        &'dot;remote;help;show'= {
        }
        &'dot;remote;help;rename'= {
        }
        &'dot;remote;help;help'= {
        }
        &'dot;branch'= {
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
            cand list 'List all branches'
            cand create 'Create a new branch'
            cand delete 'Delete a branch'
            cand rename 'Rename a branch'
            cand set-upstream 'Set upstream tracking for a branch'
            cand unset-upstream 'Remove upstream tracking for a branch'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'dot;branch;list'= {
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;branch;create'= {
            cand -f 'Starting point (commit or branch)'
            cand --from 'Starting point (commit or branch)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;branch;delete'= {
            cand -f 'Force deletion'
            cand --force 'Force deletion'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;branch;rename'= {
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;branch;set-upstream'= {
            cand -b 'Branch name (current branch if not specified)'
            cand --branch 'Branch name (current branch if not specified)'
            cand -b 'Remote branch name (same as local branch if not specified)'
            cand --remote-branch 'Remote branch name (same as local branch if not specified)'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;branch;unset-upstream'= {
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;branch;help'= {
            cand list 'List all branches'
            cand create 'Create a new branch'
            cand delete 'Delete a branch'
            cand rename 'Rename a branch'
            cand set-upstream 'Set upstream tracking for a branch'
            cand unset-upstream 'Remove upstream tracking for a branch'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'dot;branch;help;list'= {
        }
        &'dot;branch;help;create'= {
        }
        &'dot;branch;help;delete'= {
        }
        &'dot;branch;help;rename'= {
        }
        &'dot;branch;help;set-upstream'= {
        }
        &'dot;branch;help;unset-upstream'= {
        }
        &'dot;branch;help;help'= {
        }
        &'dot;config'= {
            cand --unset 'Unset the configuration key'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;completion'= {
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;help'= {
            cand add 'Add files to be tracked'
            cand status 'Show the working tree status'
            cand commit 'Record changes to the repository'
            cand checkout 'Switch branches or restore working tree files'
            cand reset 'Reset current HEAD to the specified state'
            cand push 'Update remote refs along with associated objects'
            cand pull 'Fetch from and integrate with another repository'
            cand init 'Initialize a new dotman repository'
            cand show 'Show various types of objects'
            cand log 'Show commit logs'
            cand diff 'Show changes between commits'
            cand rm 'Remove files from tracking'
            cand remote 'Manage remote repositories'
            cand branch 'Manage branches'
            cand config 'Get and set repository or user options'
            cand completion 'Generate shell completion scripts'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'dot;help;add'= {
        }
        &'dot;help;status'= {
        }
        &'dot;help;commit'= {
        }
        &'dot;help;checkout'= {
        }
        &'dot;help;reset'= {
        }
        &'dot;help;push'= {
        }
        &'dot;help;pull'= {
        }
        &'dot;help;init'= {
        }
        &'dot;help;show'= {
        }
        &'dot;help;log'= {
        }
        &'dot;help;diff'= {
        }
        &'dot;help;rm'= {
        }
        &'dot;help;remote'= {
            cand list 'List all remotes'
            cand add 'Add a new remote'
            cand remove 'Remove a remote'
            cand set-url 'Set the URL for a remote'
            cand show 'Show information about a remote'
            cand rename 'Rename a remote'
        }
        &'dot;help;remote;list'= {
        }
        &'dot;help;remote;add'= {
        }
        &'dot;help;remote;remove'= {
        }
        &'dot;help;remote;set-url'= {
        }
        &'dot;help;remote;show'= {
        }
        &'dot;help;remote;rename'= {
        }
        &'dot;help;branch'= {
            cand list 'List all branches'
            cand create 'Create a new branch'
            cand delete 'Delete a branch'
            cand rename 'Rename a branch'
            cand set-upstream 'Set upstream tracking for a branch'
            cand unset-upstream 'Remove upstream tracking for a branch'
        }
        &'dot;help;branch;list'= {
        }
        &'dot;help;branch;create'= {
        }
        &'dot;help;branch;delete'= {
        }
        &'dot;help;branch;rename'= {
        }
        &'dot;help;branch;set-upstream'= {
        }
        &'dot;help;branch;unset-upstream'= {
        }
        &'dot;help;config'= {
        }
        &'dot;help;completion'= {
        }
        &'dot;help;help'= {
        }
    ]
    $completions[$command]
}
